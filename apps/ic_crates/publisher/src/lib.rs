//! Controller-operated Cargo HTTP bridge. Provider transfer has one selected owner.

#![doc = include_str!("../README.md")]

mod config;
mod registry;
mod server;
mod upload;
mod workflow;

use config::Configuration;
use registry::IcRegistry;
use std::path::Path;
use workflow::Publisher;

#[derive(Debug)]
struct Error {
    status: u16,
    detail: &'static str,
}
impl Error {
    fn bad(detail: &'static str) -> Self {
        Self {
            status: 400,
            detail,
        }
    }
    fn unavailable(detail: &'static str) -> Self {
        Self {
            status: 503,
            detail,
        }
    }
}
type Result<T> = std::result::Result<T, Error>;

fn save(path: &Path, bytes: &[u8]) -> Result<()> {
    ic_host_fs::durable::create_private_bytes_with_parents(path, bytes).map_err(|_| {
        Error::unavailable("Durable job write failed; preserve and inspect original evidence.")
    })
}

/// Run a loopback HTTP bridge using one explicit configuration file.
/// TLS, ingress timeouts and request concurrency belong to the front proxy.
pub fn run(path: &Path) -> std::result::Result<(), String> {
    start(path).map_err(|error| error.detail.to_owned())
}

fn start(path: &Path) -> Result<()> {
    let bytes = ic_host_fs::read::read_file(path, 16384)
        .map_err(|_| Error::bad("Cannot read bridge configuration."))?;
    let config: Configuration =
        serde_json::from_slice(&bytes).map_err(|_| Error::bad("Invalid bridge configuration."))?;
    config.validate()?;
    // All parents are operator-custodied, with no untrusted writers or symlinks.
    let metadata = std::fs::symlink_metadata(&config.spool)
        .map_err(|_| Error::bad("Create a private spool directory before startup."))?;
    if !metadata.is_dir() {
        return Err(Error::bad("Spool must be a real directory."));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::bad("Spool must have mode 0700."));
        }
    }
    let token = ic_host_fs::read::read_private_bytes::<32>(&config.token_file)
        .map_err(|_| Error::bad("Token file must contain 32 private bytes."))?
        .ok_or_else(|| Error::bad("Missing token file."))?;
    let registry = IcRegistry::new(&config)?;
    let publisher = Publisher {
        config,
        actor: registry.actor.to_text(),
        root_digest: registry.root_digest.clone(),
        registry,
    };
    let server = tiny_http::Server::http(publisher.config.listen)
        .map_err(|_| Error::bad("Cannot bind loopback listener."))?;
    eprintln!(
        "IC Crates bridge listening on {}; index {}/index/",
        publisher.config.listen, publisher.config.public_origin
    );
    let expected = ic_crates_protocol::checksum_hex(&token);
    for request in server.incoming_requests() {
        server::handle(request, &publisher, &expected);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
