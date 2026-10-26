use crate::{Error, Result};
use candid::Principal;
use ic_platform::agent::{BasicIdentity, Identity};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectedFile {
    pub path: PathBuf,
    pub sha256: String,
}

impl SelectedFile {
    pub fn verify(&self) -> Result<()> {
        if !self.path.is_absolute() {
            return Err(Error::bad("Worker tool paths must be absolute."));
        }
        let identity = ic_host_fs::read::hash_file(&self.path, 600 * 1024 * 1024)
            .map_err(|_| Error::bad("Cannot identify worker tool."))?;
        if identity.sha256.to_string() != self.sha256 {
            return Err(Error::bad(
                "Worker tool bytes differ from the selected hash.",
            ));
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Configuration {
    pub blob_cli: SelectedFile,
    pub blob_version: String,
    pub node: SelectedFile,
    pub sdk_module: SelectedFile,
    pub launcher_module: SelectedFile,
    pub playwright_module: SelectedFile,
    pub chromium: SelectedFile,
    pub host_bundle: SelectedFile,
    pub worker_bundle: SelectedFile,
    pub uploader_identity: PathBuf,
    pub verifier_identity: PathBuf,
    pub browser_identity: PathBuf,
    pub verifier: Principal,
    pub project: String,
    pub bucket: String,
    pub provider_origin: String,
    pub asset_port: u16,
    pub max_body_bytes: u64,
    pub max_requests: usize,
    pub max_request_bytes: usize,
    pub max_total_request_bytes: usize,
    pub browser_seconds: u64,
}

impl Configuration {
    pub fn read(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err(Error::bad("Worker configuration path must be absolute."));
        }
        let bytes = ic_host_fs::read::read_file(path, 16 * 1024)
            .map_err(|_| Error::bad("Cannot read worker configuration."))?;
        serde_json::from_slice(&bytes).map_err(|_| Error::bad("Invalid worker configuration."))
    }

    pub fn validate(&self, uploader: Principal) -> Result<()> {
        if self.asset_port == 0
            || !(1..=240).contains(&self.browser_seconds)
            || !(1..=1024 * 1024 * 1024).contains(&self.max_body_bytes)
            || self.verifier == Principal::anonymous()
            || self.verifier == Principal::management_canister()
        {
            return Err(Error::bad("Invalid worker budgets or verifier."));
        }
        let origin = url::Url::parse(&self.provider_origin)
            .map_err(|_| Error::bad("Invalid provider origin."))?;
        if origin.scheme() != "https"
            || origin.origin().ascii_serialization() != self.provider_origin
        {
            return Err(Error::bad("Use an HTTPS provider origin."));
        }
        for file in [
            &self.blob_cli,
            &self.node,
            &self.sdk_module,
            &self.launcher_module,
            &self.playwright_module,
            &self.chromium,
            &self.host_bundle,
            &self.worker_bundle,
        ] {
            file.verify()?;
        }
        identity(&self.uploader_identity, uploader)?;
        identity(&self.verifier_identity, self.verifier)?;
        if !self.browser_identity.is_absolute() {
            return Err(Error::bad("Browser signer path must be absolute."));
        }
        Ok(())
    }
}

fn identity(path: &Path, expected: Principal) -> Result<()> {
    if !path.is_absolute() {
        return Err(Error::bad("Worker signer paths must be absolute."));
    }
    let bytes = ic_host_fs::read::read_file(path, 8192)
        .map_err(|_| Error::bad("Cannot read worker identity."))?;
    let signer = BasicIdentity::from_pem(&bytes[..])
        .map_err(|_| Error::bad("Invalid worker Ed25519 PEM."))?;
    if signer
        .sender()
        .map_err(|_| Error::bad("Invalid worker signer."))?
        != expected
    {
        return Err(Error::bad(
            "Worker signer differs from its selected principal.",
        ));
    }
    Ok(())
}
