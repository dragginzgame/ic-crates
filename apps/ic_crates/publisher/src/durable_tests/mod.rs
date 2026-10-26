//! Consumer publication and descriptor custody against the selected Host APIs.

#![cfg(unix)]

use super::{save, workflow};
use ic_host_process::tool::{OutputLimit, OutputLimits, capture_group_command};
use std::{fs, os::unix::fs::PermissionsExt, process::Command, time::Duration};

#[test]
fn job_publication_is_private_and_never_replaces_retained_evidence() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("job/original/intent.json");
    save(&path, b"original").unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(save(&path, b"replacement").unwrap_err().status, 503);
    assert_eq!(fs::read(&path).unwrap(), b"original");

    let blocked = root.path().join("blocked-parent");
    fs::write(&blocked, b"retained").unwrap();
    assert_eq!(
        save(&blocked.join("child"), b"new").unwrap_err().status,
        503
    );
    assert_eq!(fs::read(&blocked).unwrap(), b"retained");

    let alias = root.path().join("occupied-link");
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    assert_eq!(save(&alias, b"replacement").unwrap_err().status, 503);
    assert_eq!(fs::read(&path).unwrap(), b"original");
}

#[test]
fn spool_lock_child() {
    let Some(path) = std::env::var_os("IC_CRATES_LOCK_TEST_PATH") else {
        return;
    };
    let lock = workflow::lock_spool(std::path::Path::new(&path));
    if std::env::var_os("IC_CRATES_LOCK_TEST_BUSY").is_some() {
        let error = lock.unwrap_err();
        assert_eq!(error.status, 503);
        assert_eq!(error.detail, "Upload spool is busy or unavailable.");
    } else {
        drop(lock.unwrap());
    }
}

#[test]
fn spool_descriptor_refuses_other_processes_until_its_owner_drops_it() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("owner.lock");
    fs::write(&path, b"retained lock bytes").unwrap();
    let mut lock = Some(workflow::lock_spool(&path).unwrap());
    for busy in [true, false] {
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args(["--exact", "durable_tests::spool_lock_child", "--nocapture"])
            .env("IC_CRATES_LOCK_TEST_PATH", &path)
            .env_remove("IC_CRATES_LOCK_TEST_BUSY");
        if busy {
            child.env("IC_CRATES_LOCK_TEST_BUSY", "1");
        }
        let result = capture_group_command(
            &mut child,
            OutputLimits {
                stdout: OutputLimit::Terminate(16 * 1024),
                stderr: OutputLimit::Terminate(16 * 1024),
                timeout: Some(Duration::from_secs(5)),
            },
        )
        .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(fs::read(&path).unwrap(), b"retained lock bytes");
        if busy {
            // Keep the same descriptor alive across the complete first child.
            assert_eq!(lock.as_ref().unwrap().metadata().unwrap().len(), 19);
            drop(lock.take());
        }
    }
}
