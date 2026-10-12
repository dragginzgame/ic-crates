//! Durable registry configuration and immutable version catalog.

use icydb::model::prelude::*;

#[canister(memory_namespace = "ic_crates_registry")]
pub struct RegistryCanister {}

#[store(canister = "RegistryCanister", storage(journaled(key = "catalog")))]
pub struct RegistryStore {}

/// Singleton explicit blob scope. It cannot be changed after installation.
#[entity(
    store = "RegistryStore",
    version = 1,
    pk(field = "id"),
    fields(
        field(name = "id", value(item(prim = "Nat64"))),
        field(name = "blob_service", value(item(prim = "Principal"))),
        field(name = "completion_verifier", value(item(prim = "Principal"))),
        field(name = "namespace", value(item(prim = "Nat128"))),
        field(name = "provider_project", value(item(prim = "Text", max_len = 256))),
        field(name = "provider_origin", value(item(prim = "Text", max_len = 256)))
    )
)]
pub struct ConfigurationRecord {}

/// Exact upload intent is durable before the registry grants uploader permission.
#[entity(
    store = "RegistryStore",
    version = 1,
    pk(field = "id"),
    fields(
        field(name = "id", value(item(prim = "Nat128"))),
        field(name = "permission", value(item(prim = "Blob", max_len = 8192)))
    )
)]
pub struct UploadRecord {}

/// One canonical crate name and semver, retaining exact publication evidence.
#[entity(
    store = "RegistryStore", version = 1, pk(field = "id"),
    index(fields = ["crate_key", "version"], unique),
    fields(
        field(name = "id", value(item(prim = "Ulid")), generated(insert = "Ulid::generate")),
        field(name = "crate_key", value(item(prim = "Text", max_len = 64))),
        field(name = "name", value(item(prim = "Text", max_len = 64))),
        field(name = "version", value(item(prim = "Text", max_len = 128))),
        field(name = "index_json", value(item(prim = "Text", max_len = 65536))),
        field(name = "permission", value(item(prim = "Blob", max_len = 8192))),
        field(name = "download_url", value(item(prim = "Text", max_len = 2048))),
        field(name = "yanked", value(item(prim = "Bool")))
    )
)]
pub struct VersionRecord {}
