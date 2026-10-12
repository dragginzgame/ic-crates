//! Cargo sparse-index entries. Dependency registry URLs are preserved explicitly.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::RegistryError;

/// Maximum serialized metadata for a single version.
pub const MAX_INDEX_BYTES: usize = 64 * 1024;

/// One dependency in Cargo's index format (distinct from publish metadata).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub name: String,
    pub req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<String>,
    pub registry: Option<String>,
    pub package: Option<String>,
}

/// A complete newline-delimited version entry consumed by Cargo.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndexEntry {
    pub name: String,
    pub vers: String,
    pub deps: Vec<Dependency>,
    pub cksum: String,
    pub features: BTreeMap<String, Vec<String>>,
    pub yanked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rust_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features2: Option<BTreeMap<String, Vec<String>>>,
}

impl IndexEntry {
    /// Parse a bounded entry and enforce the initial registry's naming/version rules.
    pub fn parse(json: &str) -> Result<Self, RegistryError> {
        if json.len() > MAX_INDEX_BYTES {
            return Err(RegistryError::Limit);
        }
        let entry: Self = serde_json::from_str(json).map_err(|_| RegistryError::InvalidMetadata)?;
        canonical_name(&entry.name)?;
        if !entry.name.bytes().all(|c| !c.is_ascii_uppercase()) {
            return Err(RegistryError::InvalidName);
        }
        let version =
            semver::Version::parse(&entry.vers).map_err(|_| RegistryError::InvalidVersion)?;
        // Build metadata creates equivalent semver versions with different archive URLs.
        if !version.build.is_empty() || version.to_string() != entry.vers {
            return Err(RegistryError::InvalidVersion);
        }
        if entry.cksum.len() != 64
            || !entry
                .cksum
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(RegistryError::InvalidChecksum);
        }
        if entry.yanked || !matches!(entry.v, None | Some(1) | Some(2)) {
            return Err(RegistryError::InvalidMetadata);
        }
        if entry.features2.is_some() && entry.v != Some(2) {
            return Err(RegistryError::InvalidMetadata);
        }
        if let Some(msrv) = &entry.rust_version {
            // Cargo allows a two-component rust_version as well as a full version.
            let msrv = if msrv.split('.').count() == 2 {
                format!("{msrv}.0")
            } else {
                msrv.clone()
            };
            let parsed =
                semver::Version::parse(&msrv).map_err(|_| RegistryError::InvalidMetadata)?;
            if !parsed.pre.is_empty() || !parsed.build.is_empty() {
                return Err(RegistryError::InvalidMetadata);
            }
        }
        if entry.deps.len() > 256 {
            return Err(RegistryError::Limit);
        }
        for dep in &entry.deps {
            canonical_name(&dep.name)?;
            if let Some(package) = &dep.package {
                canonical_name(package)?;
            }
            semver::VersionReq::parse(&dep.req).map_err(|_| RegistryError::InvalidMetadata)?;
            if !matches!(dep.kind.as_deref(), None | Some("normal" | "dev" | "build")) {
                return Err(RegistryError::InvalidMetadata);
            }
            if let Some(registry) = &dep.registry {
                let url = url::Url::parse(registry).map_err(|_| RegistryError::InvalidMetadata)?;
                if url.scheme() != "https"
                    || url.host_str().is_none()
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(RegistryError::InvalidMetadata);
                }
            }
        }
        Ok(entry)
    }

    /// Serialize one version as a complete sparse-index line.
    pub fn line(&self) -> Result<String, RegistryError> {
        let mut line = serde_json::to_string(self).map_err(|_| RegistryError::InvalidMetadata)?;
        line.push('\n');
        if line.len() > MAX_INDEX_BYTES {
            return Err(RegistryError::Limit);
        }
        Ok(line)
    }
}

/// Namespace key. Case and hyphen/underscore variants cannot claim separate crates.
pub fn canonical_name(name: &str) -> Result<String, RegistryError> {
    if name.is_empty()
        || name.len() > 64
        || !name.as_bytes()[0].is_ascii_alphabetic()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(RegistryError::InvalidName);
    }
    Ok(name.to_ascii_lowercase().replace('_', "-"))
}

/// Cargo's index sharding path uses lowercase spelling, preserving underscores.
pub fn index_path(name: &str) -> Result<String, RegistryError> {
    canonical_name(name)?;
    let name = name.to_ascii_lowercase();
    Ok(match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_paths_preserve_spelling_and_namespace_keys_prevent_collisions() {
        for (name, path) in [
            ("a", "1/a"),
            ("ab", "2/ab"),
            ("abc", "3/a/abc"),
            ("serde_json", "se/rd/serde_json"),
        ] {
            assert_eq!(index_path(name).unwrap(), path);
        }
        assert_eq!(
            canonical_name("My_crate").unwrap(),
            canonical_name("my-crate").unwrap()
        );
        for name in ["", "../a", "éa", "1abc", "a/b"] {
            assert_eq!(index_path(name), Err(RegistryError::InvalidName));
        }
    }

    #[test]
    fn index_roundtrip_preserves_dependency_registry_features_and_rename() {
        let json = format!(
            r#"{{"name":"demo","vers":"1.2.3","deps":[{{"name":"random","package":"rand","req":"^0.8","features":[],"optional":false,"default_features":true,"target":null,"kind":"normal","registry":"https://github.com/rust-lang/crates.io-index"}}],"cksum":"{}","features":{{}},"features2":{{"extra":["dep:random"]}},"v":2,"yanked":false}}"#,
            "ab".repeat(32)
        );
        let entry = IndexEntry::parse(&json).unwrap();
        assert_eq!(
            IndexEntry::parse(entry.line().unwrap().trim()).unwrap(),
            entry
        );
        assert_eq!(
            IndexEntry::parse(&json.replace("1.2.3", "1.2.3+build")),
            Err(RegistryError::InvalidVersion)
        );
        assert_eq!(
            IndexEntry::parse(&json.replace("\"v\":2", "\"v\":1")),
            Err(RegistryError::InvalidMetadata)
        );
        assert_eq!(
            IndexEntry::parse(&json.replace(&"ab".repeat(32), &"AB".repeat(32))),
            Err(RegistryError::InvalidChecksum)
        );
    }
}
