//! A surviving original job owns upload permission, worker dispatch and retries.

use crate::{Error, Result, config::Configuration, registry::Registry, save, upload::Upload};
use ic_blob_storage_contracts::dto::{
    reference::ReferenceUpload,
    upload::{
        admission::UploadAdmissionRequest,
        manifest::{UploadManifestDeclaration, UploadManifestHeader, UploadManifestRequest},
    },
};
use ic_crates_protocol::{IndexEntry, PublishRequest, canonical_name};
use ic_host_fs::durable::{PublicationMode, WriteOptions, write_with};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Intent {
    configuration: String,
    actor: String,
    root_digest: String,
    worker_digest: String,
    index: IndexEntry,
    bytes: u64,
}

pub(crate) struct Publisher<B> {
    pub config: Configuration,
    pub registry: B,
    pub actor: String,
    pub root_digest: String,
}

impl<B: Registry> Publisher<B> {
    pub fn publish(&self, mut upload: Upload) -> Result<()> {
        let _lock = ic_host_fs::durable::try_lock_regular_file_with_parents(
            &self.config.spool.join("owner.lock"),
        )
        .map_err(|_| Error::unavailable("Upload spool is busy or unavailable."))?;
        let job = self.config.spool.join(format!(
            "{}@{}",
            canonical_name(&upload.index.name).map_err(|_| Error::bad("Invalid crate name."))?,
            upload.index.vers
        ));
        let worker_digest = ic_host_fs::read::hash_file(&self.config.worker, 64 * 1024 * 1024)
            .map_err(|_| Error::unavailable("Cannot identify selected worker."))?;
        let intent = Intent {
            configuration: serde_json::to_string(&self.config)
                .map_err(|_| Error::bad("Invalid scope."))?,
            actor: self.actor.clone(),
            root_digest: self.root_digest.clone(),
            worker_digest: worker_digest.sha256.to_string(),
            index: upload.index.clone(),
            bytes: upload.bytes,
        };
        match fs::symlink_metadata(&job) {
            Ok(metadata) => {
                if !metadata.is_dir() {
                    return Err(Error::unavailable("Invalid retained job."));
                }
                let bytes = ic_host_fs::read::read_file(&job.join("intent.json"), 128 * 1024)
                    .map_err(|_| {
                        Error::unavailable(
                            "Original job is incomplete; inspect it before proceeding.",
                        )
                    })?;
                let original: Intent = serde_json::from_slice(&bytes)
                    .map_err(|_| Error::unavailable("Invalid retained job."))?;
                if original != intent {
                    return Err(Error {
                        status: 409,
                        detail: "Original job has different content or scope; preserve it.",
                    });
                }
                // A replay never grants again and never invokes the worker. Only
                // the canister may authenticate completion and the exact catalog commit.
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.capacity(upload.bytes)?;
                let permission = self.permission(&upload)?;
                let manifest = UploadManifestRequest {
                    permission,
                    declaration: UploadManifestDeclaration {
                        chunks: upload
                            .manifest
                            .manifest()
                            .chunks()
                            .iter()
                            .map(|hash| *hash.as_bytes())
                            .collect(),
                        headers: vec![
                            UploadManifestHeader {
                                name: "Content-Length".into(),
                                value: upload.bytes.to_string(),
                            },
                            UploadManifestHeader {
                                name: "Content-Type".into(),
                                value: "application/octet-stream".into(),
                            },
                        ],
                    },
                };
                upload
                    .file
                    .seek(SeekFrom::Start(0))
                    .map_err(|_| Error::unavailable("Cannot preserve archive."))?;
                write_with(
                    &job.join("archive.crate"),
                    WriteOptions {
                        mode: PublicationMode::CreateNew,
                        permissions: 0o600,
                    },
                    |file| std::io::copy(upload.file.as_file_mut(), file),
                )
                .map_err(|_| Error::unavailable("Cannot preserve archive; inspect spool."))?;
                save(
                    &job.join("permission.candid"),
                    &candid::encode_one(permission)
                        .map_err(|_| Error::bad("Invalid permission."))?,
                )?;
                save(
                    &job.join("manifest.candid"),
                    &candid::encode_one(manifest).map_err(|_| Error::bad("Invalid manifest."))?,
                )?;
                save(
                    &job.join("publish.candid"),
                    &candid::encode_one(PublishRequest {
                        index_json: upload
                            .index
                            .line()
                            .map_err(|_| Error::bad("Invalid index."))?,
                        permission,
                    })
                    .map_err(|_| Error::bad("Invalid publication."))?,
                )?;
                save(
                    &job.join("intent.json"),
                    &serde_json::to_vec(&intent).map_err(|_| Error::bad("Invalid intent."))?,
                )?;
                // The complete immutable intent survives before either dispatch.
                save(
                    &job.join("admission-claimed"),
                    b"Original admission may be dispatched; never repeat it.\n",
                )?;
                self.registry.admit(permission, &job)?;
                save(
                    &job.join("worker-claimed"),
                    b"Original worker may be dispatched; never repeat it.\n",
                )?;
                self.transfer(&job)?;
            }
            Err(_) => return Err(Error::unavailable("Cannot inspect original job.")),
        }
        let bytes = ic_host_fs::read::read_file(&job.join("publish.candid"), 72 * 1024)
            .map_err(|_| Error::unavailable("Cannot inspect original publication."))?;
        let request: PublishRequest = candid::decode_one(&bytes)
            .map_err(|_| Error::unavailable("Invalid original publication."))?;
        if request.index_json
            != upload
                .index
                .line()
                .map_err(|_| Error::bad("Invalid index."))?
            || request.permission.upload.service != self.config.blob_service
            || request.permission.upload.tenant != self.config.registry
            || request.permission.upload.namespace != self.config.namespace
            || request.permission.uploader != self.config.uploader
            || request.permission.upload.root != *upload.manifest.hashes().provider_root.as_bytes()
            || request.permission.upload.bytes != upload.bytes
        {
            return Err(Error::unavailable("Original publication binding changed."));
        }
        let attempt = self.attempt(&job)?;
        self.registry.publish(request, &attempt)?;
        save(
            &attempt.join("confirmed"),
            b"The IC catalog confirmed this exact publication.\n",
        )
    }

    fn capacity(&self, incoming: u64) -> Result<()> {
        let mut jobs = 0_usize;
        let mut bytes = incoming;
        for entry in fs::read_dir(&self.config.spool)
            .map_err(|_| Error::unavailable("Cannot inspect spool capacity."))?
        {
            let path = entry
                .map_err(|_| Error::unavailable("Cannot inspect spool capacity."))?
                .path();
            if path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().contains('@'))
            {
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| Error::unavailable("Cannot inspect job."))?;
                if !metadata.is_dir() {
                    return Err(Error::unavailable("Invalid retained job path."));
                }
                jobs += 1;
                let archive = fs::symlink_metadata(path.join("archive.crate"))
                    .map_err(|_| Error::unavailable("Incomplete retained archive."))?;
                if !archive.is_file() {
                    return Err(Error::unavailable("Invalid retained archive."));
                }
                bytes = bytes
                    .checked_add(archive.len())
                    .ok_or_else(|| Error::unavailable("Spool capacity overflow."))?;
            }
        }
        if jobs >= self.config.max_retained_jobs || bytes > self.config.max_spool_bytes {
            return Err(Error::unavailable(
                "Retained job/archive capacity exhausted; keep original evidence.",
            ));
        }
        Ok(())
    }

    fn permission(&self, upload: &Upload) -> Result<UploadAdmissionRequest> {
        fn id() -> Result<u128> {
            let mut bytes = [0; 16];
            getrandom::fill(&mut bytes)
                .map_err(|_| Error::unavailable("Cannot allocate upload identity."))?;
            let id = u128::from_be_bytes(bytes);
            if id == 0 {
                return Err(Error::unavailable("Invalid random identity."));
            }
            Ok(id)
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::bad("Invalid host clock."))?
            .as_nanos();
        let expires_at_ns =
            u64::try_from(now + u128::from(self.config.permission_seconds) * 1_000_000_000)
                .map_err(|_| Error::bad("Host clock exceeds permission range."))?;
        Ok(UploadAdmissionRequest {
            upload: ReferenceUpload {
                service: self.config.blob_service,
                tenant: self.config.registry,
                namespace: self.config.namespace,
                upload: id()?,
                object: id()?,
                incarnation: 1,
                first_reference: id()?,
                root: *upload.manifest.hashes().provider_root.as_bytes(),
                bytes: upload.bytes,
            },
            uploader: self.config.uploader,
            expires_at_ns,
        })
    }

    fn attempt(&self, job: &Path) -> Result<PathBuf> {
        for number in 0..100 {
            let path = job.join(format!("catalog-{number:03}"));
            match fs::create_dir(&path) {
                Ok(()) => {
                    save(
                        &path.join("claim"),
                        b"Exact catalog publication observation/commit attempt.\n",
                    )?;
                    return Ok(path);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(Error::unavailable("Cannot claim catalog attempt.")),
            }
        }
        Err(Error::unavailable(
            "Catalog attempt limit reached; inspect existing evidence.",
        ))
    }

    fn transfer(&self, job: &Path) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::unavailable("Cannot start worker runtime."))?;
        runtime.block_on(async {
            let mut command = tokio::process::Command::new(&self.config.worker);
            // The worker owns its provider/browser journals and child lifecycle.
            // No request text enters a shell, argument or environment value.
            let mut child = command
                .arg(job)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|_| Error::unavailable("Worker did not start; preserve its claim."))?;
            let status = tokio::time::timeout(
                Duration::from_secs(self.config.worker_timeout_seconds),
                child.wait(),
            )
            .await
            .map_err(|_| {
                Error::unavailable("Worker timed out; inspect original provider journals.")
            })?
            .map_err(|_| {
                Error::unavailable("Worker outcome is uncertain; preserve original journals.")
            })?;
            if !status.success() {
                return Err(Error::unavailable(
                    "Worker refused or failed; inspect original provider journals.",
                ));
            }
            Ok(())
        })
    }
}
