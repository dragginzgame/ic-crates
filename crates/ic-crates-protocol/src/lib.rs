//! Cargo wire formats and bounded publication rules, independent of IC execution.

#![doc = include_str!("../README.md")]

mod cargo;
mod http;
mod index;
mod publication;

pub use cargo::{CargoDependency, CargoMetadata};
pub use http::{HttpRequest, HttpResponse};
pub use index::{Dependency, IndexEntry, MAX_INDEX_BYTES, canonical_name, index_path};
pub use publication::{PublishRequest, RegistryConfiguration, RegistryError, checksum_hex};
