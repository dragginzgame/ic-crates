//! Cargo's upload metadata is a distinct external contract from index entries.

use crate::{Dependency, IndexEntry, RegistryError};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Publish API metadata; presentation fields and future Cargo fields are ignored.
#[derive(Clone, Debug, Deserialize)]
pub struct CargoMetadata {
    pub name: String,
    pub vers: String,
    pub deps: Vec<CargoDependency>,
    pub features: BTreeMap<String, Vec<String>>,
    pub links: Option<String>,
    pub rust_version: Option<String>,
}

/// Dependency spelling supplied by Cargo's publish API.
#[derive(Clone, Debug, Deserialize)]
pub struct CargoDependency {
    pub name: String,
    pub version_req: String,
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: Option<String>,
    pub registry: Option<String>,
    pub explicit_name_in_toml: Option<String>,
}

impl CargoMetadata {
    /// Translate renamed dependencies and extended features; compute checksum at the host.
    pub fn into_index(self, checksum: String) -> Result<IndexEntry, RegistryError> {
        let (features2, features): (BTreeMap<_, _>, BTreeMap<_, _>) =
            self.features.into_iter().partition(|(_, values)| {
                values
                    .iter()
                    .any(|value| value.starts_with("dep:") || value.contains("?/"))
            });
        let entry = IndexEntry {
            name: self.name,
            vers: self.vers,
            deps: self
                .deps
                .into_iter()
                .map(|dep| Dependency {
                    name: dep
                        .explicit_name_in_toml
                        .clone()
                        .unwrap_or_else(|| dep.name.clone()),
                    package: dep.explicit_name_in_toml.map(|_| dep.name),
                    req: dep.version_req,
                    features: dep.features,
                    optional: dep.optional,
                    default_features: dep.default_features,
                    target: dep.target,
                    kind: dep.kind,
                    registry: dep.registry,
                })
                .collect(),
            cksum: checksum,
            features,
            yanked: false,
            links: self.links,
            rust_version: self.rust_version,
            v: (!features2.is_empty()).then_some(2),
            features2: (!features2.is_empty()).then_some(features2),
        };
        IndexEntry::parse(entry.line()?.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_translation_preserves_registry_target_kind_and_feature_syntax() {
        let metadata: CargoMetadata = serde_json::from_value(serde_json::json!({
            "name":"demo", "vers":"0.1.0", "deps":[{
                "name":"serde", "explicit_name_in_toml":"codec", "version_req":"^1",
                "features":["derive"], "optional":true, "default_features":false,
                "target":"cfg(unix)", "kind":"build",
                "registry":"https://github.com/rust-lang/crates.io-index"
            }], "features":{"plain":[], "codec":["dep:codec", "codec?/derive"]},
            "description":"ignored presentation field", "future_cargo_field":true
        }))
        .unwrap();
        let entry = metadata.into_index("ab".repeat(32)).unwrap();
        assert_eq!(entry.deps[0].name, "codec");
        assert_eq!(entry.deps[0].package.as_deref(), Some("serde"));
        assert_eq!(entry.deps[0].req, "^1");
        assert_eq!(entry.deps[0].kind.as_deref(), Some("build"));
        assert_eq!(entry.deps[0].target.as_deref(), Some("cfg(unix)"));
        assert_eq!(
            entry.deps[0].registry.as_deref(),
            Some("https://github.com/rust-lang/crates.io-index")
        );
        assert!(!entry.deps[0].default_features);
        assert_eq!(entry.v, Some(2));
        assert!(entry.features2.unwrap().contains_key("codec"));
        assert!(entry.features.contains_key("plain"));
    }
}
