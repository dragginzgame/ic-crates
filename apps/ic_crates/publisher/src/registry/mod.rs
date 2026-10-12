//! Authenticated replica transport; retain signed intent before mutation.

use crate::{Error, Result, config::Configuration, save};
use candid::{CandidType, Principal};
use ic_agent::{Agent, Identity, agent::CallResponse, identity::BasicIdentity};
use ic_blob_storage_contracts::dto::upload::admission::{
    UploadAdmissionMutation, UploadAdmissionRequest,
};
use ic_crates_protocol::{HttpRequest, HttpResponse, PublishRequest, RegistryError};
use std::{path::Path, time::Duration};

pub(crate) trait Registry {
    fn admit(&self, permission: UploadAdmissionRequest, job: &Path) -> Result<()>;
    fn publish(&self, request: PublishRequest, attempt: &Path) -> Result<()>;
    fn read(&self, path: &str) -> Result<HttpResponse>;
}

pub(crate) struct IcRegistry {
    agent: Agent,
    runtime: tokio::runtime::Runtime,
    registry: Principal,
    pub actor: Principal,
    pub root_digest: String,
}

impl IcRegistry {
    pub fn new(config: &Configuration) -> Result<Self> {
        let identity_bytes = ic_host_fs::read::read_file(&config.identity_file, 8192)
            .map_err(|_| Error::bad("Cannot read Ed25519 identity."))?;
        let identity = BasicIdentity::from_pem(identity_bytes.as_slice())
            .map_err(|_| Error::bad("Expected an Ed25519 PEM identity."))?;
        let actor = identity
            .sender()
            .map_err(|_| Error::bad("Invalid signing identity."))?;
        let agent = Agent::builder()
            .with_url(&config.replica_url)
            .with_identity(identity)
            .with_max_tcp_error_retries(0)
            .with_max_response_body_size(2 * 1024 * 1024)
            .with_max_concurrent_requests(1)
            .build()
            .map_err(|_| Error::bad("Cannot build replica transport."))?;
        if let Some(path) = &config.root_key_file {
            let key = ic_host_fs::read::read_file(path, 4096)
                .map_err(|_| Error::bad("Cannot read selected root key."))?;
            if key.is_empty() {
                return Err(Error::bad("Empty root key."));
            }
            agent.set_root_key(key);
        }
        let digest =
            ic_blob_storage_contracts::identity::ContentDigest::compute(&agent.read_root_key());
        Ok(Self {
            agent,
            actor,
            registry: config.registry,
            root_digest: ic_crates_protocol::checksum_hex(digest.as_bytes()),
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| Error::unavailable("Cannot start replica runtime."))?,
        })
    }

    fn update(&self, method: &str, args: Vec<u8>, journal: Option<&Path>) -> Result<Vec<u8>> {
        let signed = self
            .agent
            .update(&self.registry, method)
            .with_arg(args)
            .sign()
            .map_err(|_| Error::unavailable("Cannot sign catalog request."))?;
        if let Some(path) = journal {
            save(
                &path.join("signed-update.json"),
                &serde_json::to_vec(&signed)
                    .map_err(|_| Error::unavailable("Cannot encode signed intent."))?,
            )?;
        }
        let response = self
            .runtime
            .block_on(async {
                tokio::time::timeout(Duration::from_secs(90), async {
                    match self
                        .agent
                        .update_signed(self.registry, signed.signed_update)
                        .await?
                    {
                        CallResponse::Response(bytes) => Ok(bytes),
                        CallResponse::Poll(id) => self
                            .agent
                            .wait(&id, self.registry)
                            .await
                            .map(|(bytes, _)| bytes),
                    }
                })
                .await
            })
            .map_err(|_| {
                Error::unavailable("Catalog response is uncertain; retain the original job.")
            })?
            .map_err(|_| {
                Error::unavailable("Catalog response is uncertain; retain the original job.")
            })?;
        if let Some(path) = journal {
            save(&path.join("reply.candid"), &response)?;
        }
        Ok(response)
    }
}

fn encode<T: CandidType>(value: T) -> Result<Vec<u8>> {
    candid::encode_one(value).map_err(|_| Error::bad("Invalid Candid request."))
}

fn decode<'a, T: candid::Deserialize<'a> + CandidType>(bytes: &'a [u8]) -> Result<T> {
    let mut limits = candid::de::DecoderConfig::new();
    limits
        .set_decoding_quota(4_000_000)
        .set_skipping_quota(100_000);
    candid::decode_one_with_config(bytes, &limits)
        .map_err(|_| Error::unavailable("Invalid catalog reply."))
}

impl Registry for IcRegistry {
    fn admit(&self, permission: UploadAdmissionRequest, job: &Path) -> Result<()> {
        let bytes = self.update(
            "registry_admit_upload",
            encode(permission)?,
            Some(&job.join("admission")),
        )?;
        let result: std::result::Result<UploadAdmissionMutation, RegistryError> = decode(&bytes)?;
        let response = result.map_err(|_| {
            Error::unavailable("Catalog refused admission; retain the original job.")
        })?;
        if response.admission.permission != permission {
            return Err(Error::unavailable("Admission binding mismatch."));
        }
        Ok(())
    }
    fn publish(&self, request: PublishRequest, attempt: &Path) -> Result<()> {
        let bytes = self.update("registry_publish", encode(request)?, Some(attempt))?;
        let result: std::result::Result<(), RegistryError> = decode(&bytes)?;
        result.map_err(|error| match error {
            RegistryError::VersionConflict | RegistryError::NameConflict => Error {
                status: 409,
                detail: "Crate name/version is already bound to different content.",
            },
            _ => Error::unavailable(
                "Publication is unconfirmed; inspect the retained job and Blob Storage evidence.",
            ),
        })
    }
    fn read(&self, path: &str) -> Result<HttpResponse> {
        decode(&self.update(
            "http_request_update",
            encode(HttpRequest {
                method: "GET".into(),
                url: path.into(),
                headers: Vec::new(),
                body: Vec::new(),
                certificate_version: None,
            })?,
            None,
        )?)
    }
}
