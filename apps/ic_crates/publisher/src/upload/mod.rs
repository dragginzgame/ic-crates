//! Bounded Cargo framing and archive inspection, with no extraction or execution.

use crate::{Error, Result};
use flate2::bufread::GzDecoder;
use ic_blob_storage_contracts::identity::caffeine::{
    CaffeineHashLimits, CaffeineHeader,
    manifest::builder::{CaffeineBuiltManifest, CaffeineManifestBuilder},
};
use ic_crates_protocol::{CargoMetadata, IndexEntry, checksum_hex};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom, Write},
    num::{NonZeroU64, NonZeroUsize},
    path::Component,
};
use tempfile::NamedTempFile;

pub(crate) struct Upload {
    pub file: NamedTempFile,
    pub index: IndexEntry,
    pub manifest: CaffeineBuiltManifest,
    pub bytes: u64,
}

fn length(input: &mut impl Read) -> Result<u64> {
    let mut bytes = [0; 4];
    input
        .read_exact(&mut bytes)
        .map_err(|_| Error::bad("Truncated Cargo upload."))?;
    Ok(u32::from_le_bytes(bytes) as u64)
}

pub(crate) fn receive(
    input: &mut impl Read,
    max_bytes: u64,
    spool: &std::path::Path,
    registry_index: &str,
) -> Result<Upload> {
    let metadata_bytes = length(input)?;
    if metadata_bytes == 0 || metadata_bytes > 256 * 1024 {
        return Err(Error::bad("Cargo metadata exceeds 256 KiB."));
    }
    let mut metadata = vec![0; metadata_bytes as usize];
    input
        .read_exact(&mut metadata)
        .map_err(|_| Error::bad("Truncated metadata."))?;
    let mut metadata: CargoMetadata =
        serde_json::from_slice(&metadata).map_err(|_| Error::bad("Invalid Cargo metadata."))?;
    for dependency in &mut metadata.deps {
        if dependency.registry.as_deref() == Some(registry_index) {
            dependency.registry = None;
        }
    }
    // Validate names and versions before opening an archive or choosing a job path.
    metadata
        .clone()
        .into_index("00".repeat(32))
        .map_err(|_| Error::bad("Invalid package metadata."))?;
    receive_archive(input, metadata, max_bytes, spool)
}

fn receive_archive(
    input: &mut impl Read,
    metadata: CargoMetadata,
    max_bytes: u64,
    spool: &std::path::Path,
) -> Result<Upload> {
    let bytes = length(input)?;
    if bytes == 0 || bytes > max_bytes {
        return Err(Error::bad("Archive exceeds the configured limit."));
    }
    let content_length = bytes.to_string();
    let headers = [
        CaffeineHeader {
            name: "Content-Length",
            value: &content_length,
        },
        CaffeineHeader {
            name: "Content-Type",
            value: "application/octet-stream",
        },
    ];
    let mut builder = CaffeineManifestBuilder::new(
        bytes,
        &headers,
        CaffeineHashLimits {
            max_content_bytes: NonZeroU64::new(max_bytes)
                .ok_or_else(|| Error::bad("Invalid archive limit."))?,
            max_append_bytes: NonZeroUsize::new(65536).unwrap(),
            max_headers: NonZeroUsize::new(2).unwrap(),
            max_header_bytes: NonZeroUsize::new(256).unwrap(),
        },
        NonZeroUsize::new(1024).unwrap(),
    )
    .map_err(|_| Error::bad("Invalid provider manifest."))?;
    let mut file =
        NamedTempFile::new_in(spool).map_err(|_| Error::unavailable("Cannot stage archive."))?;
    let mut buffer = [0; 65536];
    while builder.remaining_bytes() > 0 {
        let count = builder.remaining_bytes().min(buffer.len() as u64) as usize;
        input
            .read_exact(&mut buffer[..count])
            .map_err(|_| Error::bad("Truncated archive."))?;
        file.write_all(&buffer[..count])
            .map_err(|_| Error::unavailable("Cannot stage archive."))?;
        builder
            .append(builder.received_bytes(), &buffer[..count])
            .map_err(|_| Error::bad("Archive hashing failed."))?;
    }
    let mut trailing = [0];
    if input
        .read(&mut trailing)
        .map_err(|_| Error::bad("Invalid upload EOF."))?
        != 0
    {
        return Err(Error::bad("Unexpected bytes after archive."));
    }
    let manifest = builder
        .finish()
        .map_err(|_| Error::bad("Archive hashing failed."))?;
    let digest: [u8; 32] = manifest.hashes().content_digest.as_bytes().to_owned();
    let index = metadata
        .into_index(checksum_hex(&digest))
        .map_err(|_| Error::bad("Invalid index metadata."))?;
    inspect(file.as_file_mut(), &index)?;
    Ok(Upload {
        file,
        index,
        manifest,
        bytes,
    })
}

fn inspect(file: &mut File, index: &IndexEntry) -> Result<()> {
    file.seek(SeekFrom::Start(0))
        .map_err(|_| Error::bad("Cannot inspect archive."))?;
    // Limit the decompressed tar stream, including header/extension work.
    let decoder = GzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(decoder.take(256 * 1024 * 1024 + 1));
    let root = format!("{}-{}", index.name, index.vers);
    let mut paths = BTreeSet::new();
    let mut total = 0_u64;
    let mut found_manifest = false;
    for entry in archive
        .entries()
        .map_err(|_| Error::bad("Invalid crate archive."))?
    {
        let mut entry = entry.map_err(|_| Error::bad("Invalid tar entry."))?;
        if paths.len() >= 4096 {
            return Err(Error::bad("Too many archive entries."));
        }
        let path = entry
            .path()
            .map_err(|_| Error::bad("Invalid archive path."))?
            .into_owned();
        let mut components = path.components();
        if components.next() != Some(Component::Normal(root.as_ref()))
            || components.any(|c| !matches!(c, Component::Normal(_)))
            || !paths.insert(path.clone())
            || (!entry.header().entry_type().is_file() && !entry.header().entry_type().is_dir())
        {
            return Err(Error::bad("Unsafe or duplicate archive path/type."));
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| Error::bad("Archive size overflow."))?;
        if total > 256 * 1024 * 1024 {
            return Err(Error::bad("Expanded archive exceeds 256 MiB."));
        }
        if path == std::path::Path::new(&root).join("Cargo.toml") {
            if entry.size() > 256 * 1024 {
                return Err(Error::bad("Manifest exceeds 256 KiB."));
            }
            let mut text = String::new();
            entry
                .read_to_string(&mut text)
                .map_err(|_| Error::bad("Invalid package manifest."))?;
            let manifest: toml::Value =
                toml::from_str(&text).map_err(|_| Error::bad("Invalid package manifest."))?;
            let package = manifest
                .get("package")
                .ok_or_else(|| Error::bad("Missing package manifest."))?;
            if package.get("name").and_then(toml::Value::as_str) != Some(&index.name)
                || package.get("version").and_then(toml::Value::as_str) != Some(&index.vers)
                || package.get("links").and_then(toml::Value::as_str) != index.links.as_deref()
                || package.get("rust-version").and_then(toml::Value::as_str)
                    != index.rust_version.as_deref()
            {
                return Err(Error::bad(
                    "Archive manifest disagrees with Cargo metadata.",
                ));
            }
            found_manifest = true;
        }
    }
    let mut decoded = archive.into_inner();
    // Require gzip CRC/footer success and enforce the entire expanded byte budget.
    std::io::copy(&mut decoded, &mut std::io::sink())
        .map_err(|_| Error::bad("Invalid gzip footer."))?;
    if decoded.limit() == 0 || !found_manifest {
        return Err(Error::bad("Invalid or oversized crate archive."));
    }
    let mut compressed = decoded.into_inner().into_inner();
    if compressed
        .read(&mut [0])
        .map_err(|_| Error::bad("Cannot inspect archive EOF."))?
        != 0
    {
        return Err(Error::bad("Unexpected bytes after gzip stream."));
    }
    Ok(())
}
