//! Canic lifecycle owner and authenticated registry endpoints; IcyDB owns the catalog.

mod design;
mod http;
mod ops;
mod workflow;

use canic::prelude::*;
use ic_blob_storage_contracts::dto::upload::admission::{
    UploadAdmissionMutation, UploadAdmissionRequest, UploadAdmissionResponse,
};
use ic_crates_protocol::{PublishRequest, RegistryConfiguration, RegistryError};

const DATABASE_AUTHORITY: &str = "icydb.ic_crates_registry";
const DATABASE_PREFIX: &str = "icydb.ic_crates_registry.";

canic::memory::memory_allocation_pool!(
    authorities = [(DATABASE_AUTHORITY, DATABASE_PREFIX)],
    exclusions = [],
);

fn icydb_memory_pool()
-> Result<icydb::db::MemoryAllocationPool, icydb::db::MemoryAllocationPoolError> {
    canic::memory::pool::framework_pool(
        vec![icydb::db::MemoryAuthority::new(
            DATABASE_AUTHORITY,
            DATABASE_PREFIX,
        )?],
        Vec::new(),
    )
}

canic::memory::memory_bootstrap_admission!(
    identity = canic::memory::admission::PolicyIdentity::new("icydb.logical-memory-admission", 1)
        .expect("constant valid IcyDB admission identity"),
    prepare = icydb::db::prepare_memory_bootstrap,
);

icydb::start!(participant);
canic::start!(lifecycle_participant(
    init = initialize_database,
    post_upgrade = restore_database,
),);

fn initialize_database() {
    crate::__icydb_lifecycle_participant::init();
}
fn restore_database() {
    crate::__icydb_lifecycle_participant::post_upgrade();
}

#[expect(clippy::unused_async, reason = "Canic deferred lifecycle signatures")]
async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
#[expect(clippy::unused_async, reason = "Canic deferred lifecycle signatures")]
async fn canic_upgrade() {}

/// Pin the storage owner, namespace, provider origin and verifier once.
#[canic_update(requires(caller::is_controller()), on_access_denied = "reject")]
fn registry_configure(config: RegistryConfiguration) -> Result<(), RegistryError> {
    icydb::db::with_request_execution(|| ops::configure(&config))
}

/// Retain an exact upload intent and grant its external uploader through the blob service.
#[canic_update(requires(caller::is_controller()), on_access_denied = "reject")]
async fn registry_admit_upload(
    permission: UploadAdmissionRequest,
) -> Result<UploadAdmissionMutation, RegistryError> {
    icydb::db::with_request_execution_async(workflow::admit_upload(permission)).await
}

/// Inspect one saved admission after an interrupted or uncertain reply.
#[canic_update(requires(caller::is_controller()), on_access_denied = "reject")]
async fn registry_upload_admission(
    permission: UploadAdmissionRequest,
) -> Result<UploadAdmissionResponse, RegistryError> {
    icydb::db::with_request_execution_async(workflow::inspect_upload(permission)).await
}

/// Commit a verified, currently live blob as one immutable Cargo version.
#[canic_update(
    requires(caller::is_controller()),
    payload(max_bytes = 72 * 1024),
    on_access_denied = "reject"
)]
async fn registry_publish(request: PublishRequest) -> Result<(), RegistryError> {
    icydb::db::with_request_execution_async(workflow::publish(request)).await
}

/// Change dependency selection without deleting the version or its archive.
#[canic_update(requires(caller::is_controller()), on_access_denied = "reject")]
fn registry_yank(name: String, version: String, yanked: bool) -> Result<(), RegistryError> {
    icydb::db::with_request_execution(|| ops::yank(&name, &version, yanked))
}

/// Queries ask the gateway to upgrade to consensus-backed update execution.
#[ic_platform::cdk::query(
    decode_with = "http::codec::decode_request",
    encode_with = "http::codec::encode_response"
)]
#[candid::candid_method(query)]
fn http_request(_: http::HttpRequest) -> http::HttpResponse {
    http::upgrade()
}

/// Read the durable sparse index or redirect to the retained immutable blob URL.
#[ic_platform::cdk::update(
    decode_with = "http::codec::decode_request",
    encode_with = "http::codec::encode_response"
)]
#[candid::candid_method(update)]
fn http_request_update(request: http::HttpRequest) -> http::HttpResponse {
    icydb::db::with_request_execution(|| http::serve(&request))
}

canic::finish!();
