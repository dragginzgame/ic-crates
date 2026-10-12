//! Explicit host transport, custody and worker selection; no ambient IC identity.

use crate::{Error, Result};
use candid::Principal;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Configuration {
    pub listen: SocketAddr,
    pub public_origin: String,
    pub replica_url: String,
    pub root_key_file: Option<PathBuf>,
    pub identity_file: PathBuf,
    /// File containing exactly 32 private token bytes, represented as hex by Cargo.
    pub token_file: PathBuf,
    pub registry: Principal,
    pub blob_service: Principal,
    pub namespace: u128,
    pub uploader: Principal,
    pub spool: PathBuf,
    /// Operator-selected Blob Storage worker; one absolute executable, no shell.
    pub worker: PathBuf,
    pub worker_timeout_seconds: u64,
    pub permission_seconds: u64,
    pub max_archive_bytes: u64,
    pub max_retained_jobs: usize,
    pub max_spool_bytes: u64,
}

impl Configuration {
    pub fn validate(&self) -> Result<()> {
        if !self.listen.ip().is_loopback()
            || self.namespace == 0
            || self.max_archive_bytes == 0
            || self.max_archive_bytes > 1024 * 1024 * 1024
            || self.max_retained_jobs == 0
            || self.max_retained_jobs > 10000
            || self.max_spool_bytes < self.max_archive_bytes
            || !(1..=600).contains(&self.worker_timeout_seconds)
            || !(self.worker_timeout_seconds + 120..=3600).contains(&self.permission_seconds)
            || [self.registry, self.blob_service, self.uploader]
                .iter()
                .any(|p| *p == Principal::anonymous() || *p == Principal::management_canister())
            || [
                &self.identity_file,
                &self.token_file,
                &self.spool,
                &self.worker,
            ]
            .iter()
            .any(|p| !p.is_absolute())
            || self
                .root_key_file
                .as_ref()
                .is_some_and(|p| !p.is_absolute())
        {
            return Err(Error::bad("Invalid bridge scope, paths or budgets."));
        }
        for text in [&self.public_origin, &self.replica_url] {
            let url = url::Url::parse(text).map_err(|_| Error::bad("Invalid bridge URL."))?;
            let local = url.host_str().is_some_and(|h| {
                h.parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
            });
            if text.len() > 256
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || url.path() != "/"
                || url.origin().ascii_serialization() != *text
                || (url.scheme() != "https" && !(url.scheme() == "http" && local))
            {
                return Err(Error::bad("Use HTTPS origins or explicit loopback HTTP."));
            }
            if text == &self.replica_url && local != self.root_key_file.is_some() {
                return Err(Error::bad(
                    "Loopback replicas require an explicit root key; remote replicas use IC trust.",
                ));
            }
        }
        Ok(())
    }
}
