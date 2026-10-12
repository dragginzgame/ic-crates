//! Wire and recovery tests exercise real Cargo separately from IC/provider qualification.

use crate::{
    Error, Result, config::Configuration, registry::Registry, server, upload, workflow::Publisher,
};
use candid::Principal;
use flate2::{Compression, write::GzEncoder};
use ic_blob_storage_contracts::dto::upload::admission::UploadAdmissionRequest;
use ic_crates_protocol::{HttpResponse, IndexEntry, PublishRequest, index_path};
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Catalog {
    admissions: AtomicUsize,
    commits: AtomicUsize,
    lose_reply: AtomicBool,
    package: Mutex<Option<(IndexEntry, Vec<u8>)>>,
}

impl Registry for Arc<Catalog> {
    fn admit(&self, _: UploadAdmissionRequest, _: &Path) -> Result<()> {
        self.admissions.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn publish(&self, request: PublishRequest, attempt: &Path) -> Result<()> {
        let job = attempt.parent().unwrap();
        if !job.join("worker-observed").is_file() {
            return Err(Error::unavailable("Fixture has no completion evidence."));
        }
        let entry = IndexEntry::parse(request.index_json.trim()).unwrap();
        let archive = fs::read(job.join("archive.crate")).unwrap();
        let mut package = self.package.lock().unwrap();
        if let Some((previous, previous_archive)) = package.as_ref() {
            assert_eq!(previous, &entry);
            assert_eq!(previous_archive, &archive);
        } else {
            *package = Some((entry, archive));
            self.commits.fetch_add(1, Ordering::SeqCst);
        }
        if self.lose_reply.swap(false, Ordering::SeqCst) {
            return Err(Error::unavailable("Fixture lost committed reply."));
        }
        Ok(())
    }
    fn read(&self, path: &str) -> Result<HttpResponse> {
        if path == "/index/config.json" {
            return Ok(server::json(200, serde_json::json!({"dl":"fixture"})));
        }
        let package = self.package.lock().unwrap();
        if let Some((entry, archive)) = package.as_ref() {
            if path == format!("/index/{}", index_path(&entry.name).unwrap()) {
                return Ok(HttpResponse {
                    status_code: 200,
                    headers: Vec::new(),
                    body: entry.line().unwrap().into_bytes(),
                    upgrade: None,
                });
            }
            if path == format!("/api/v1/crates/{}/{}/download", entry.name, entry.vers) {
                return Ok(HttpResponse {
                    status_code: 200,
                    headers: Vec::new(),
                    body: archive.clone(),
                    upgrade: None,
                });
            }
        }
        Ok(server::json(
            404,
            serde_json::json!({"errors":[{"detail":"not found"}]}),
        ))
    }
}

fn config(root: &Path) -> Configuration {
    use std::os::unix::fs::PermissionsExt;
    let worker = root.join("worker.sh");
    fs::write(
        &worker,
        "#!/bin/sh\nprintf 'observed\\n' >> \"$1/worker-observed\"\n",
    )
    .unwrap();
    fs::set_permissions(&worker, fs::Permissions::from_mode(0o700)).unwrap();
    let spool = root.join("spool");
    fs::create_dir(&spool).unwrap();
    Configuration {
        listen: "127.0.0.1:0".parse().unwrap(),
        public_origin: "http://127.0.0.1:1".into(),
        replica_url: "https://icp-api.io".into(),
        root_key_file: None,
        identity_file: root.join("identity.pem"),
        token_file: root.join("token"),
        registry: Principal::from_slice(&[1]),
        blob_service: Principal::from_slice(&[2]),
        namespace: 1,
        uploader: Principal::from_slice(&[3]),
        spool,
        worker,
        worker_timeout_seconds: 10,
        permission_seconds: 600,
        max_archive_bytes: 10 * 1024 * 1024,
        max_retained_jobs: 10,
        max_spool_bytes: 100 * 1024 * 1024,
    }
}

fn body(version: &str, path: &str, symlink: bool) -> Vec<u8> {
    let metadata = serde_json::to_vec(
        &serde_json::json!({"name":"wire_demo", "vers":"0.1.0", "deps":[],"features":{}}),
    )
    .unwrap();
    let gzip = GzEncoder::new(Vec::new(), Compression::default());
    let mut tar = tar::Builder::new(gzip);
    let text = format!("[package]\nname=\"wire_demo\"\nversion=\"{version}\"\n");
    let mut header = tar::Header::new_gnu();
    header.set_size(text.len() as u64);
    header.set_mode(0o644);
    if symlink {
        header.set_entry_type(tar::EntryType::Symlink);
    }
    let path_bytes = path.as_bytes();
    header.as_mut_bytes()[..path_bytes.len()].copy_from_slice(path_bytes);
    header.set_cksum();
    tar.append(&header, text.as_bytes()).unwrap();
    let archive = tar.into_inner().unwrap().finish().unwrap();
    let mut body = Vec::new();
    body.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    body.extend(metadata);
    body.extend_from_slice(&(archive.len() as u32).to_le_bytes());
    body.extend(archive);
    body
}

#[test]
fn cargo_framing_and_archive_validation_rejects_untrusted_input_before_dispatch() {
    let root = tempfile::tempdir().unwrap();
    let original = body("0.1.0", "wire_demo-0.1.0/Cargo.toml", false);
    let received = upload::receive(
        &mut Cursor::new(&original),
        10 * 1024 * 1024,
        root.path(),
        "sparse+https://fixture.example/index/",
    )
    .unwrap();
    assert_eq!(received.index.name, "wire_demo");
    assert_ne!(
        received.manifest.hashes().provider_root.as_bytes(),
        received.manifest.hashes().content_digest.as_bytes()
    );
    let mut invalid = vec![
        original[..original.len() - 1].to_vec(),
        [original.clone(), vec![0]].concat(),
        body("0.2.0", "wire_demo-0.1.0/Cargo.toml", false),
        body("0.1.0", "wire_demo-0.1.0/../Cargo.toml", false),
        body("0.1.0", "wire_demo-0.1.0/Cargo.toml", true),
        u32::MAX.to_le_bytes().to_vec(),
    ];
    let mut crc = original.clone();
    let crc_index = crc.len() - 8;
    crc[crc_index] ^= 1;
    invalid.push(crc);
    for input in invalid {
        assert!(
            upload::receive(
                &mut Cursor::new(input),
                10 * 1024 * 1024,
                root.path(),
                "sparse+https://fixture.example/index/"
            )
            .is_err()
        );
    }
    assert!(
        upload::receive(
            &mut Cursor::new(original),
            1,
            root.path(),
            "sparse+https://fixture.example/index/"
        )
        .is_err()
    );
}

#[test]
fn lost_commit_reply_reuses_original_permission_and_never_restarts_worker() {
    let root = tempfile::tempdir().unwrap();
    let config = config(root.path());
    let catalog = Arc::new(Catalog::default());
    catalog.lose_reply.store(true, Ordering::SeqCst);
    let publisher = Publisher {
        config,
        registry: catalog.clone(),
        actor: "fixture".into(),
        root_digest: "fixture".into(),
    };
    let input = body("0.1.0", "wire_demo-0.1.0/Cargo.toml", false);
    let receive = || {
        upload::receive(
            &mut Cursor::new(&input),
            publisher.config.max_archive_bytes,
            &publisher.config.spool,
            "sparse+https://fixture.example/index/",
        )
        .unwrap()
    };
    assert!(publisher.publish(receive()).is_err());
    let job = publisher.config.spool.join("wire-demo@0.1.0");
    let permission = fs::read(job.join("permission.candid")).unwrap();
    publisher.publish(receive()).unwrap();
    assert_eq!(catalog.admissions.load(Ordering::SeqCst), 1);
    assert_eq!(catalog.commits.load(Ordering::SeqCst), 1);
    assert_eq!(fs::read(job.join("permission.candid")).unwrap(), permission);
    assert_eq!(
        fs::read_to_string(job.join("worker-observed")).unwrap(),
        "observed\n"
    );
    let mut changed = receive();
    changed.index.links = Some("changed".into());
    assert_eq!(publisher.publish(changed).unwrap_err().status, 409);
    fs::remove_file(job.join("intent.json")).unwrap();
    assert!(publisher.publish(receive()).is_err());
    assert_eq!(catalog.admissions.load(Ordering::SeqCst), 1);
}

#[test]
fn failed_worker_is_retained_and_replay_only_checks_original_completion() {
    let root = tempfile::tempdir().unwrap();
    let config = config(root.path());
    fs::write(
        &config.worker,
        "#!/bin/sh\nprintf 'claimed\\n' >> \"$1/worker-count\"\nexit 1\n",
    )
    .unwrap();
    let catalog = Arc::new(Catalog::default());
    let publisher = Publisher {
        config,
        registry: catalog.clone(),
        actor: "fixture".into(),
        root_digest: "fixture".into(),
    };
    let input = body("0.1.0", "wire_demo-0.1.0/Cargo.toml", false);
    for _ in 0..2 {
        let upload = upload::receive(
            &mut Cursor::new(&input),
            publisher.config.max_archive_bytes,
            &publisher.config.spool,
            "sparse+https://fixture.example/index/",
        )
        .unwrap();
        assert!(publisher.publish(upload).is_err());
    }
    let job = publisher.config.spool.join("wire-demo@0.1.0");
    assert_eq!(
        fs::read_to_string(job.join("worker-count")).unwrap(),
        "claimed\n"
    );
    assert_eq!(catalog.admissions.load(Ordering::SeqCst), 1);
    assert_eq!(catalog.commits.load(Ordering::SeqCst), 0);
}

#[test]
fn unmodified_cargo_publishes_then_resolves_downloads_and_builds_from_the_bridge() {
    let root = tempfile::tempdir().unwrap();
    let mut config = config(root.path());
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let address = server.server_addr().to_ip().unwrap();
    config.listen = address;
    config.public_origin = format!("http://{address}");
    let catalog = Arc::new(Catalog::default());
    let observed = catalog.clone();
    let publisher = Publisher {
        config,
        registry: catalog,
        actor: "fixture".into(),
        root_digest: "fixture".into(),
    };
    let stop = Arc::new(AtomicBool::new(false));
    let ending = stop.clone();
    let worker = std::thread::spawn(move || {
        while !ending.load(Ordering::SeqCst) {
            if let Some(request) = server.recv_timeout(Duration::from_millis(100)).unwrap() {
                server::handle(request, &publisher, &"12".repeat(32));
            }
        }
    });
    // Explicitly untracked, local fixture source and credentials; no real registry.
    let home = root.path().join("cargo-home");
    fs::create_dir(&home).unwrap();
    fs::write(home.join("config.toml"), format!("[registry]\nglobal-credential-providers=[\"cargo:token\"]\n[registries.fixture]\nindex=\"sparse+http://{address}/index/\"\n")).unwrap();
    let package = root.path().join("package");
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(package.join("Cargo.toml"), "[package]\nname=\"wire_demo\"\nversion=\"0.1.0\"\nedition=\"2024\"\nlicense=\"MIT\"\ndescription=\"local wire fixture\"\n").unwrap();
    fs::write(
        package.join("src/lib.rs"),
        "pub fn answer() -> u32 { 42 }\n",
    )
    .unwrap();
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target/cargo-wire")
        .join(root.path().file_name().unwrap());
    let cargo = |dir: &Path, args: &[&str]| {
        let output = Command::new("cargo")
            .args(args)
            .current_dir(dir)
            .env("CARGO_HOME", &home)
            .env("CARGO_TARGET_DIR", &target)
            .env("CARGO_REGISTRIES_FIXTURE_TOKEN", "12".repeat(32))
            .env("CARGO_HTTP_TIMEOUT", "15")
            .env("CARGO_HTTP_MULTIPLEXING", "false")
            .env("CARGO_HTTP_PROXY", "")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Cargo failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    // Real Cargo upload framing and follow-up sparse polling.
    cargo(
        &package,
        &["publish", "--registry", "fixture", "--allow-dirty"],
    );
    let consumer = root.path().join("consumer");
    fs::create_dir_all(consumer.join("src")).unwrap();
    fs::write(consumer.join("Cargo.toml"), "[package]\nname=\"wire_consumer\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[dependencies]\nwire_demo={version=\"0.1.0\",registry=\"fixture\"}\n").unwrap();
    fs::write(
        consumer.join("src/main.rs"),
        "fn main() { assert_eq!(wire_demo::answer(), 42); }\n",
    )
    .unwrap();
    cargo(&consumer, &["run", "--quiet"]);
    // Token rejection occurs before an archive is admitted.
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(b"PUT /api/v1/crates/new HTTP/1.1\r\nHost: fixture\r\nAuthorization: invalid\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 403"));
    stop.store(true, Ordering::SeqCst);
    worker.join().unwrap();
    assert_eq!(observed.admissions.load(Ordering::SeqCst), 1);
    assert_eq!(observed.commits.load(Ordering::SeqCst), 1);
}
