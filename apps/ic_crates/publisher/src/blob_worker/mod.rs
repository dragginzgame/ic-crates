//! Compose original Cargo jobs with the maintained Blob CLI and browser owners.

mod config;
#[cfg(test)]
mod tests;

use crate::{Error, Result, save, workflow::Intent};
use candid::Principal;
use config::Configuration;
use ic_blob_storage_contracts::{
    dto::upload::{admission::UploadAdmissionRequest, manifest::UploadManifestRequest},
    identity::{ProviderRootHash, caffeine::manifest::CaffeineManifestLimits},
    provider::preparation::{PreparedManifestLimits, decode_prepared_manifest},
};
use ic_crates_protocol::{PublishRequest, checksum_hex};
use ic_host_process::tool::{OutputLimit, OutputLimits, capture_group_command};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// Compose one already-admitted job with the selected upstream worker tools.
/// An existing worker directory refuses redispatch; upstream journals own recovery.
pub fn run_blob_worker(configuration: &Path, job: &Path) -> std::result::Result<(), String> {
    run(configuration, job).map_err(|error| error.detail.to_owned())
}

/// Verify original inputs and real SDK preparation without service/provider calls.
/// Output must be a new absolute directory; the original job is never changed.
pub fn preflight_blob_worker(
    configuration: &Path,
    job: &Path,
    output: &Path,
) -> std::result::Result<(), String> {
    prepare(configuration, job, output)
        .map(|_| ())
        .map_err(|error| error.detail.to_owned())
}

struct Prepared {
    config: Configuration,
    bridge: crate::config::Configuration,
    permission: UploadAdmissionRequest,
    input: PathBuf,
    root: PathBuf,
    network: &'static str,
    digest: String,
}

fn run(configuration: &Path, job: &Path) -> Result<()> {
    let prepared = prepare(configuration, job, &job.join("blob-worker"))?;
    let prepare_dir = prepared.root.join("preparation");
    let report = cli(
        &prepared,
        "prepare-upload",
        &prepared.config.uploader_identity,
        prepared.permission.uploader,
        &[
            "--request".into(),
            job.join("manifest.candid").into_os_string(),
            "--run-dir".into(),
            prepare_dir.clone().into_os_string(),
        ],
    )?;
    if report["outcome"] != "acknowledged" || report["authentication"] != "ic_update_certificate" {
        return Err(Error::unavailable(
            "Blob preparation is not acknowledged; retain original journals.",
        ));
    }
    let response = ic_host_fs::read::read_file(&prepare_dir.join("response.candid"), 64 * 1024)
        .map_err(|_| Error::unavailable("Missing original preparation reply."))?;
    let request: UploadManifestRequest = read_candid(&job.join("manifest.candid"), 64 * 1024)?;
    ic_blob_storage_contracts::upload::manifests::reply::mutation(
        &request,
        &response,
        ic_blob_storage_contracts::upload::manifests::reply::UploadManifestReplyLimits {
            max_reply_bytes: (64 * 1024).try_into().unwrap(),
            declaration: manifest_limits(prepared.permission.upload.bytes),
        },
    )
    .map_err(|_| {
        Error::unavailable("Original preparation reply differs from the selected manifest.")
    })?;
    let report = node(&prepared, "upload", prepared.config.browser_seconds + 10)?;
    if report["state"] != "transfer-observed"
        || report["retry_authorized"] != false
        || report["service_completion_checked"] != false
    {
        return Err(Error::unavailable(
            "Browser transfer is incomplete; retain its original profile.",
        ));
    }
    let observation = prepared.root.join("observation");
    let report = cli(
        &prepared,
        "observe-upload",
        &prepared.config.verifier_identity,
        prepared.config.verifier,
        &[
            "--permission".into(),
            job.join("permission.candid").into_os_string(),
            "--gateway".into(),
            prepared.config.provider_origin.clone().into(),
            "--max-bytes".into(),
            prepared.permission.upload.bytes.to_string().into(),
            "--run-dir".into(),
            observation.clone().into_os_string(),
        ],
    )?;
    let statement: ic_blob_storage_contracts::dto::upload::completion::UploadAttestationRequest =
        read_candid(&observation.join("statement.candid"), 4096)?;
    if statement.permission != prepared.permission
        || checksum_hex(&statement.content_digest) != prepared.digest
        || report["attestation_dispatched"] != false
    {
        return Err(Error::unavailable(
            "Verifier observation differs from the original Cargo archive.",
        ));
    }
    let report = cli(
        &prepared,
        "submit-attestation",
        &prepared.config.verifier_identity,
        prepared.config.verifier,
        &["--run-dir".into(), observation.into_os_string()],
    )?;
    if report["outcome"] != "accepted" || report["retry_authorized"] != false {
        return Err(Error::unavailable(
            "Attestation is pending or refused; inspect original signed intent.",
        ));
    }
    save(
        &prepared.root.join("completed.json"),
        &serde_json::to_vec(&json!({
            "schema":1, "attestation_accepted":true, "catalog_publication_checked":false,
            "retry_authorized":false,
        }))
        .unwrap(),
    )?;
    Ok(())
}

fn prepare(configuration: &Path, job: &Path, output: &Path) -> Result<Prepared> {
    if !job.is_absolute() || !output.is_absolute() || output.symlink_metadata().is_ok() {
        return Err(Error::bad(
            "Use an absolute original job and a new worker evidence directory.",
        ));
    }
    let config = Configuration::read(configuration)?;
    let intent: Intent = serde_json::from_slice(
        &ic_host_fs::read::read_file(&job.join("intent.json"), 128 * 1024)
            .map_err(|_| Error::bad("Cannot inspect original Cargo intent."))?,
    )
    .map_err(|_| Error::bad("Invalid original Cargo intent."))?;
    let bridge: crate::config::Configuration = serde_json::from_str(&intent.configuration)
        .map_err(|_| Error::bad("Invalid original bridge scope."))?;
    bridge.validate()?;
    config.validate(bridge.uploader)?;
    if config.browser_seconds + 240 > bridge.worker_timeout_seconds
        || intent.bytes == 0
        || intent.bytes > config.max_body_bytes
    {
        return Err(Error::bad(
            "Worker budgets exceed the original bridge deadline or body limit.",
        ));
    }
    if ic_host_fs::read::hash_file(&bridge.worker, crate::config::MAX_WORKER_BYTES)
        .map_err(|_| Error::bad("Cannot identify the original selected worker."))?
        .sha256
        .to_string()
        != intent.worker_digest
    {
        return Err(Error::bad(
            "Selected worker differs from the original Cargo intent.",
        ));
    }
    let permission: UploadAdmissionRequest = read_candid(&job.join("permission.candid"), 4096)?;
    let manifest: UploadManifestRequest = read_candid(&job.join("manifest.candid"), 64 * 1024)?;
    let publish: PublishRequest = read_candid(&job.join("publish.candid"), 72 * 1024)?;
    if manifest.permission != permission
        || publish.permission != permission
        || publish.index_json
            != intent
                .index
                .line()
                .map_err(|_| Error::bad("Invalid original index."))?
        || permission.upload.service != bridge.blob_service
        || permission.upload.tenant != bridge.registry
        || permission.upload.namespace != bridge.namespace
        || permission.uploader != bridge.uploader
        || permission.upload.bytes != intent.bytes
    {
        return Err(Error::bad("Original worker scope or permission differs."));
    }
    let index_line = intent
        .index
        .line()
        .map_err(|_| Error::bad("Invalid original index."))?;
    ic_crates_protocol::IndexEntry::parse(&index_line)
        .map_err(|_| Error::bad("Invalid original index."))?;
    let identity = ic_host_fs::read::hash_file(&job.join("archive.crate"), config.max_body_bytes)
        .map_err(|_| Error::bad("Cannot inspect original archive."))?;
    if identity.bytes != intent.bytes || identity.sha256.to_string() != intent.index.cksum {
        return Err(Error::bad("Original archive bytes or checksum differ."));
    }
    let agent = ic_agent::Agent::builder()
        .with_url(&bridge.replica_url)
        .build()
        .map_err(|_| Error::bad("Invalid original replica."))?;
    if let Some(path) = &bridge.root_key_file {
        agent.set_root_key(
            ic_host_fs::read::read_file(path, 1024)
                .map_err(|_| Error::bad("Cannot read original IC trust root."))?,
        );
    }
    let key = agent.read_root_key();
    if checksum_hex(ic_blob_storage_contracts::identity::ContentDigest::compute(&key).as_bytes())
        != intent.root_digest
    {
        return Err(Error::bad(
            "Worker IC trust root differs from the original bridge.",
        ));
    }
    let binding = json!({"key":format!("{}:{}:{}", permission.upload.service, permission.upload.tenant, permission.upload.upload),
        "service":permission.upload.service.to_text(), "tenant":permission.upload.tenant.to_text(),
        "uploader":permission.uploader.to_text(), "operation":permission.upload.upload.to_string(),
        "root":ProviderRootHash::try_from(permission.upload.root.as_slice()).map_err(|_| Error::bad("Invalid original provider root."))?.to_string(),
        "project":config.project,"bucket":config.bucket,"permission":candid::encode_one(permission).unwrap()});
    let input = output.join("input.json");
    save(
        &output.join("selection.json"),
        &serde_json::to_vec(&config).unwrap(),
    )?;
    save(&output.join("browser.mjs"), include_bytes!("browser.mjs"))?;
    save(&input, &serde_json::to_vec(&json!({"configuration":config, "binding":binding,
        "body":job.join("archive.crate"), "body_sha256":intent.index.cksum, "bytes":intent.bytes.to_string(),
        "replica_url":bridge.replica_url, "root_key":key, "profile":output.join("browser-profile")})).unwrap())?;
    let mut prepared = Prepared {
        config,
        network: if bridge.root_key_file.is_some() {
            "local"
        } else {
            "ic"
        },
        bridge,
        permission,
        input,
        root: output.to_owned(),
        digest: intent.index.cksum,
    };
    let version = execute(
        &prepared.root,
        "blob-version",
        Command::new(&prepared.config.blob_cli.path).arg("--version"),
        5,
    )?;
    if version["tool"] != "blob-storage" || version["version"] != prepared.config.blob_version {
        return Err(Error::bad(
            "Blob CLI version differs from the selected release.",
        ));
    }
    let report = node(&prepared, "prepare", 30)?;
    let expected_bytes = intent.bytes.to_string();
    if report["body_sha256"] != prepared.digest
        || report["bytes"].as_str() != Some(expected_bytes.as_str())
        || report["uploader"] != permission.uploader.to_text()
        || report["provider_requests"] != 0
    {
        return Err(Error::bad("SDK preparation differs from original inputs."));
    }
    let sdk = report["manifest_json"]
        .as_str()
        .ok_or_else(|| Error::bad("Missing SDK manifest."))?;
    let declaration = decode_prepared_manifest(
        ProviderRootHash::try_from(permission.upload.root.as_slice())
            .map_err(|_| Error::bad("Invalid original provider root."))?,
        intent.bytes,
        sdk.as_bytes(),
        PreparedManifestLimits {
            max_json_bytes: (256 * 1024).try_into().unwrap(),
            manifest: manifest_limits(intent.bytes),
        },
    )
    .map_err(|_| Error::bad("SDK manifest differs from the original native manifest."))?;
    if declaration != manifest.declaration {
        return Err(Error::bad(
            "SDK headers or ordered leaves differ from the original declaration.",
        ));
    }
    let mut value: Value = serde_json::from_slice(
        &ic_host_fs::read::read_file(&prepared.input, 300 * 1024)
            .map_err(|_| Error::bad("Cannot inspect prepared browser input."))?,
    )
    .unwrap();
    value["manifest_json"] = sdk.into();
    // The final browser input has a distinct immutable name, before any effect.
    prepared.input = prepared.root.join("transfer.json");
    save(&prepared.input, &serde_json::to_vec(&value).unwrap())?;
    save(
        &prepared.root.join("preflight.json"),
        &serde_json::to_vec(&json!({"schema":1,
        "original_inputs_verified":true,"sdk_manifest_verified":true,"service_requests":0,
        "provider_requests":0,"retry_authorized":false}))
        .unwrap(),
    )?;
    Ok(prepared)
}

fn manifest_limits(bytes: u64) -> CaffeineManifestLimits {
    CaffeineManifestLimits {
        max_content_bytes: bytes.try_into().unwrap(),
        max_chunks: 1024.try_into().unwrap(),
        max_headers: 16.try_into().unwrap(),
        max_header_bytes: 4096.try_into().unwrap(),
    }
}

fn read_candid<T: candid::CandidType + for<'de> serde::Deserialize<'de>>(
    path: &Path,
    maximum: usize,
) -> Result<T> {
    let bytes = ic_host_fs::read::read_file(path, maximum)
        .map_err(|_| Error::bad("Cannot inspect original worker request."))?;
    let value: T =
        candid::decode_one(&bytes).map_err(|_| Error::bad("Invalid original worker request."))?;
    if candid::encode_one(&value).map_err(|_| Error::bad("Invalid canonical worker request."))?
        != bytes
    {
        return Err(Error::bad(
            "Worker request is not the original canonical Candid.",
        ));
    }
    Ok(value)
}

fn node(prepared: &Prepared, mode: &str, seconds: u64) -> Result<Value> {
    prepared.config.validate(prepared.permission.uploader)?;
    execute(
        &prepared.root,
        &format!("browser-{mode}"),
        Command::new(&prepared.config.node.path)
            .arg(prepared.root.join("browser.mjs"))
            .arg(mode)
            .arg(&prepared.input),
        seconds,
    )
}

fn cli(
    prepared: &Prepared,
    operation: &str,
    identity: &Path,
    actor: Principal,
    args: &[std::ffi::OsString],
) -> Result<Value> {
    prepared.config.validate(prepared.permission.uploader)?;
    let mut command = Command::new(&prepared.config.blob_cli.path);
    command
        .args([
            operation,
            "--network",
            prepared.network,
            "--url",
            &prepared.bridge.replica_url,
            "--identity",
        ])
        .arg(identity)
        .args([
            "--actor",
            &actor.to_text(),
            "--service",
            &prepared.bridge.blob_service.to_text(),
            "--namespace",
            &prepared.bridge.namespace.to_string(),
        ])
        .args(args);
    if let Some(path) = &prepared.bridge.root_key_file {
        command.arg("--root-key").arg(path);
    }
    execute(&prepared.root, operation, &mut command, 35)
}

fn execute(root: &Path, label: &str, command: &mut Command, seconds: u64) -> Result<Value> {
    command.current_dir(root).env_clear();
    for key in ["HOME", "PATH", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let result = capture_group_command(
        command,
        OutputLimits {
            stdout: OutputLimit::Terminate(300 * 1024),
            stderr: OutputLimit::Terminate(64 * 1024),
            timeout: Some(Duration::from_secs(seconds)),
        },
    );
    match result {
        Ok(result) => {
            save(&root.join(format!("{label}.stdout")), &result.stdout)?;
            save(&root.join(format!("{label}.stderr")), &result.stderr)?;
            if !result.status.success() {
                return Err(Error::unavailable(
                    "Blob worker phase failed; preserve original evidence.",
                ));
            }
            serde_json::from_slice(&result.stdout)
                .map_err(|_| Error::unavailable("Invalid worker phase report."))
        }
        Err(error) => {
            if let Some(evidence) = error.evidence() {
                save(&root.join(format!("{label}.stdout")), &evidence.stdout)?;
                save(&root.join(format!("{label}.stderr")), &evidence.stderr)?;
            }
            save(
                &root.join(format!("{label}.failure")),
                error.to_string().as_bytes(),
            )?;
            Err(Error::unavailable(
                "Blob worker outcome is uncertain; preserve original evidence.",
            ))
        }
    }
}
