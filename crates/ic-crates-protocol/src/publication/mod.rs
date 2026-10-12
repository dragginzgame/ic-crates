//! Explicit installation scope and publication evidence.

use candid::{CandidType, Principal};
use ic_blob_storage_contracts::dto::upload::admission::UploadAdmissionRequest;
use serde::Deserialize;

/// Immutable registry serving configuration, installed once by a controller.
#[derive(Clone, Debug, CandidType, Deserialize, Eq, PartialEq)]
pub struct RegistryConfiguration {
    pub blob_service: Principal,
    pub completion_verifier: Principal,
    pub namespace: u128,
    pub provider_project: String,
    pub provider_origin: String,
}

impl RegistryConfiguration {
    /// Validate explicit identities and an HTTPS origin without paths or credentials.
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.namespace == 0
            || [self.blob_service, self.completion_verifier]
                .into_iter()
                .any(|p| p == Principal::anonymous() || p == Principal::management_canister())
        {
            return Err(RegistryError::InvalidConfiguration);
        }
        if self.provider_project.is_empty()
            || self.provider_project.len() > 256
            || self.provider_project.trim() != self.provider_project
            || self.provider_project.chars().any(char::is_control)
        {
            return Err(RegistryError::InvalidConfiguration);
        }
        if self.provider_origin.len() > 256 {
            return Err(RegistryError::Limit);
        }
        let origin = url::Url::parse(&self.provider_origin)
            .map_err(|_| RegistryError::InvalidConfiguration)?;
        if origin.scheme() != "https"
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.path() != "/"
            || origin.query().is_some()
            || origin.fragment().is_some()
            || origin.origin().ascii_serialization() != self.provider_origin
        {
            return Err(RegistryError::InvalidConfiguration);
        }
        Ok(())
    }
}

/// Controller's exact version metadata and original tenant-owned upload permission.
#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct PublishRequest {
    /// Index-format JSON; normal crates.io dependencies require their explicit registry URL.
    pub index_json: String,
    /// The registry canister must be this permission's tenant.
    pub permission: UploadAdmissionRequest,
}

/// Typed publication/read failures; no failure authorizes a new provider upload.
#[derive(Clone, Debug, CandidType, Deserialize, Eq, PartialEq)]
pub enum RegistryError {
    Denied,
    NotConfigured,
    ConfigurationConflict,
    InvalidConfiguration,
    InvalidName,
    InvalidVersion,
    InvalidMetadata,
    InvalidChecksum,
    BlobBinding,
    BlobUnavailable,
    ChecksumMismatch,
    UploadConflict,
    VersionConflict,
    NameConflict,
    NotFound,
    Limit,
    Database { code: u32 },
    Internal,
}

/// Cargo expects plain lowercase hex of raw archive bytes, with no `sha256:` prefix.
pub fn checksum_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_origin_cannot_inject_credentials_paths_or_redirect_headers() {
        let mut config = RegistryConfiguration {
            blob_service: Principal::from_slice(&[1]),
            completion_verifier: Principal::from_slice(&[2]),
            namespace: 1,
            provider_project: "crates".into(),
            provider_origin: "https://storage.example".into(),
        };
        assert_eq!(config.validate(), Ok(()));
        for origin in [
            "http://storage.example",
            "https://user@storage.example",
            "https://storage.example/path",
            "https://storage.example?x=1",
            "https://storage.example/",
            "https://storage.example\r\nX: y",
        ] {
            config.provider_origin = origin.into();
            assert!(config.validate().is_err());
        }
    }
}
