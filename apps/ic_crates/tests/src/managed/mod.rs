//! Actual managed Wasm, synthetic Canic Root and owner-run PocketIC; no provider calls.

use candid::Principal;
use canic::{
    ids::ReleaseBuildId,
    testing::{
        CandidCallExt, ManagedApplicationInit, ManagedComponentGroupQualificationInput,
        ManagedRoleQualificationArtifact, install_managed_component_group,
    },
};
use ic_blob_storage_contracts::dto::{
    configuration::{
        ServiceBillingInput, ServiceConfigurationInput, ServiceFundingInput,
        ServiceInstallationInput, ServiceReadInput, ServiceResourceInput,
    },
    recovery::CurrentInstanceRecoveryFailure,
    reference::ReferenceUpload,
    tenant::{TenantEnrollmentResponse, TenantFailure, TenantScope, TenantUpdateRequest},
    upload::admission::{UploadAdmissionMutation, UploadAdmissionRequest, UploadAdmissionResponse},
};
use ic_crates_protocol::{
    HttpRequest, HttpResponse, PublishRequest, RegistryConfiguration, RegistryError,
};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    task::Poll,
    time::Duration,
};

const CONFIG: &str = include_str!("../../../canic.toml");
const NAMESPACE: u128 = 7;
fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn installation(service: Principal) -> Vec<u8> {
    candid::encode_one(ServiceInstallationInput {
        configuration: ServiceConfigurationInput {
            service,
            operator: principal(7),
            payment_account: principal(8),
            namespace: NAMESPACE,
            resources: ServiceResourceInput {
                max_tenants: 2,
                max_object_bytes: 1024,
                max_headers: 8,
                max_header_bytes: 1024,
                max_chunks: 4,
                max_tenant_chunks: 2,
                max_objects: 4,
                max_tenant_objects: 2,
                max_physical_bytes: 30000,
                max_liability_bytes: 20000,
                max_tenant_logical_bytes: 10000,
                max_references_per_object: 2,
                max_receipts_per_object: 3,
                max_active: 4,
                max_tenant_active: 2,
            },
            billing: ServiceBillingInput {
                cashier: principal(10),
                reserve: 100,
                minimum_balance: 100,
                target_balance: 200,
                max_gateway_entries: 8,
                max_gateway_unique: 4,
            },
            funding: ServiceFundingInput {
                allocated: 10000,
                renewal_ceiling: 10000,
                reserve: 100,
                max_attempts: 4,
            },
            reads: ServiceReadInput {
                sessions: 2,
                tenant_sessions: 1,
                reply_bytes: 2048,
                bytes: 4096,
                tenant_bytes: 2048,
            },
        },
        project: "ic-crates-fixture".into(),
        completion_verifier: principal(11),
    })
    .unwrap()
}

#[derive(Deserialize)]
struct BuildResult {
    schema_version: u32,
    app: String,
    event: String,
    completed: bool,
    environment: String,
    profile: String,
    role: Option<String>,
    release_build_id: ReleaseBuildId,
    release_manifest: PathBuf,
}

fn artifacts() -> (
    PathBuf,
    ReleaseBuildId,
    Vec<ManagedRoleQualificationArtifact>,
) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let app = workspace.join("apps/ic_crates");
    let path = PathBuf::from(std::env::var_os("IC_CRATES_BUILD_RESULT").expect("run make test-ic"));
    let bytes = ic_host_fs::read::read_file(&path, 16384).unwrap();
    let result: BuildResult = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result.schema_version, 1);
    assert_eq!(result.app, "ic_crates");
    assert_eq!(result.event, "build_completed");
    assert!(result.completed);
    assert_eq!(result.environment, "local");
    assert_eq!(result.profile, "fast");
    assert!(result.role.is_none());
    assert_eq!(
        std::fs::read_to_string(app.join("canic.toml")).unwrap(),
        CONFIG
    );
    canic_host::release_build::validate_finalized_release_build_manifest(
        &app,
        result.release_build_id,
        &result.release_manifest,
    )
    .unwrap();
    let snapshot =
        canic_host::release_set::AppConfigSnapshot::load(&app.join("canic.toml")).unwrap();
    let current = canic_host::release_set::load_persisted_current_release_set_manifest(
        &app,
        result.release_build_id,
    )
    .unwrap();
    assert_eq!(
        current.path.canonicalize().unwrap(),
        result.release_manifest.canonicalize().unwrap()
    );
    let union = canic_host::release_set::load_persisted_application_artifact_union(
        &app,
        snapshot.component_topology(),
        result.release_build_id,
    )
    .unwrap();
    assert_eq!(
        union.digest,
        current.manifest.application_artifact_union_sha256
    );
    assert_eq!(union.union.entries.len(), 2);
    let artifacts = union
        .union
        .entries
        .iter()
        .map(|entry| {
            let wasm_path = app.join(&entry.wasm_relative_path);
            canic_host::canister_build::validate_wasm_candid_endpoints(
                &wasm_path,
                &wasm_path.with_extension("did"),
            )
            .unwrap();
            let wasm = ic_host_fs::read::read_file(&wasm_path, 32 * 1024 * 1024).unwrap();
            assert_eq!(wasm.len() as u64, entry.wasm_size_bytes);
            let digest = ic_blob_storage_contracts::identity::ContentDigest::compute(&wasm);
            assert_eq!(
                ic_crates_protocol::checksum_hex(digest.as_bytes()),
                entry.wasm_sha256_hex
            );
            let mut artifact = ManagedRoleQualificationArtifact::new(entry.role.clone(), wasm);
            if entry.role.as_str() == "blob" {
                artifact.application_init_args = ManagedApplicationInit::ForCanister(installation);
            }
            artifact
        })
        .collect();
    (app, result.release_build_id, artifacts)
}

fn request(url: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".into(),
        url: url.into(),
        headers: Vec::new(),
        body: Vec::new(),
        certificate_version: None,
    }
}

#[test]
#[ignore = "requires matching managed Wasm and owner-run PocketIC; use make test-ic"]
fn managed_registry_blob_admission_and_same_release_restoration() {
    let (_, build, artifacts) = artifacts();
    let build_text = build.to_string();
    let fixture = install_managed_component_group(ManagedComponentGroupQualificationInput::new(
        CONFIG,
        "backend",
        &build_text,
        vec![principal(7), principal(8), principal(9)],
        artifacts,
    ))
    .expect("managed component installation");
    let registry = fixture.unique_role(&"registry".parse().unwrap()).unwrap();
    let blob = fixture.unique_role(&"blob".parse().unwrap()).unwrap();
    let root = fixture.root();
    let pic = fixture.pic();
    let operator = principal(7);
    let outsider = principal(9);
    let recover = || {
        let response: Result<(), CurrentInstanceRecoveryFailure> = pic
            .update_candid_as(blob, operator, "blob_resume_current_instance", ())
            .unwrap();
        response.unwrap();
    };
    recover();
    let config = RegistryConfiguration {
        blob_service: blob,
        completion_verifier: principal(11),
        namespace: NAMESPACE,
        provider_project: "ic-crates-fixture".into(),
        provider_origin: "https://storage.example".into(),
    };
    let denied = pic.update_candid_as::<Result<(), RegistryError>, _>(
        registry,
        outsider,
        "registry_configure",
        (config.clone(),),
    );
    assert!(
        denied.is_err(),
        "stranger cannot mutate registry configuration"
    );
    let configure = || {
        // Canic activation and IcyDB's bounded asynchronous recovery are
        // separate readiness conditions. Only the owner's pending diagnostic
        // permits another attempt at this exact idempotent configuration.
        ic_testkit::pic::tick_until(pic, 60, Duration::from_secs(1), |pic| {
            let response: Result<(), RegistryError> = pic
                .update_candid_as(registry, root, "registry_configure", (config.clone(),))
                .unwrap();
            match response {
                Ok(()) => Poll::Ready(Ok(())),
                Err(RegistryError::Database { code })
                    if code
                        == u32::from(
                            icydb::ErrorCode::RUNTIME_BOUNDARY_DATABASE_STARTUP_RECOVERY_PENDING
                                .raw(),
                        ) =>
                {
                    Poll::Pending
                }
                Err(error) => Poll::Ready(Err(error)),
            }
        })
        .expect("registry configuration must become ready within the startup budget");
    };
    configure();
    let mut conflict = config.clone();
    conflict.provider_project = "conflict".into();
    let response: Result<(), RegistryError> = pic
        .update_candid_as(registry, root, "registry_configure", (conflict,))
        .unwrap();
    assert_eq!(response, Err(RegistryError::ConfigurationConflict));
    let query: HttpResponse = pic
        .query_candid(registry, "http_request", (request("/index/config.json"),))
        .unwrap();
    assert_eq!(query.upgrade, Some(true));
    let read = |url| {
        pic.update_candid::<HttpResponse, _>(registry, "http_request_update", (request(url),))
            .unwrap()
    };
    assert_eq!(read("/index/config.json").status_code, 200);
    assert_eq!(read("/index/de/mo/demo").status_code, 404);

    let scope = TenantScope {
        service: blob,
        namespace: NAMESPACE,
        tenant: registry,
    };
    let enrollment: Result<TenantEnrollmentResponse, TenantFailure> = pic
        .update_candid_as(
            blob,
            operator,
            "blob_update_tenant",
            (TenantUpdateRequest {
                scope,
                expected: None,
                active: true,
            },),
        )
        .unwrap();
    assert!(enrollment.unwrap().enrollment.unwrap().active);
    let permission = UploadAdmissionRequest {
        upload: ReferenceUpload {
            service: blob,
            tenant: registry,
            namespace: NAMESPACE,
            upload: 1,
            object: 2,
            incarnation: 1,
            first_reference: 3,
            root: [0xab; 32],
            bytes: 1,
        },
        uploader: principal(8),
        expires_at_ns: pic
            .get_time()
            .as_nanos_since_unix_epoch()
            .checked_add(7_200_000_000_000)
            .unwrap(),
    };
    assert!(
        pic.update_candid_as::<Result<UploadAdmissionMutation, RegistryError>, _>(
            registry,
            outsider,
            "registry_admit_upload",
            (permission,)
        )
        .is_err()
    );
    let granted: Result<UploadAdmissionMutation, RegistryError> = pic
        .update_candid_as(registry, root, "registry_admit_upload", (permission,))
        .unwrap();
    let granted = granted.unwrap();
    assert_eq!(granted.admission.permission, permission);
    assert!(!granted.replayed);
    let inspect = || {
        let result: Result<UploadAdmissionResponse, RegistryError> = pic
            .update_candid_as(registry, root, "registry_upload_admission", (permission,))
            .unwrap();
        assert_eq!(result.unwrap().permission, permission);
    };
    inspect();
    let replay: Result<UploadAdmissionMutation, RegistryError> = pic
        .update_candid_as(registry, root, "registry_admit_upload", (permission,))
        .unwrap();
    assert!(replay.unwrap().replayed);
    let mut changed = permission;
    changed.uploader = outsider;
    let conflicting: Result<UploadAdmissionMutation, RegistryError> = pic
        .update_candid_as(registry, root, "registry_admit_upload", (changed,))
        .unwrap();
    assert_eq!(conflicting.unwrap_err(), RegistryError::UploadConflict);
    let index = serde_json::json!({"name":"demo", "vers":"0.1.0", "deps":[], "features":{}, "cksum":"cd".repeat(32), "yanked":false}).to_string();
    let publish = || {
        let response: Result<(), RegistryError> = pic
            .update_candid_as(
                registry,
                root,
                "registry_publish",
                (PublishRequest {
                    index_json: index.clone(),
                    permission,
                },),
            )
            .unwrap();
        assert_eq!(response, Err(RegistryError::BlobUnavailable));
        assert_eq!(read("/index/de/mo/demo").status_code, 404);
    };
    publish();
    // Registry restoration must preserve configuration and exact intents while
    // Canic retains the protected binding. No provider evidence is fabricated.
    fixture
        .upgrade_same_release(registry, Duration::from_secs(300))
        .unwrap();
    configure();
    inspect();
    publish();
    assert!(
        pic.update_candid_as::<Result<(), RegistryError>, _>(
            registry,
            outsider,
            "registry_configure",
            (config.clone(),)
        )
        .is_err()
    );
    // Blob restoration fences mutation; the installed operator's canonical
    // current-instance recovery is required before inspecting through the tenant.
    fixture
        .upgrade_same_release(blob, Duration::from_secs(300))
        .unwrap();
    recover();
    inspect();
    publish();
    println!(
        "Managed registry/blob qualification passed: controller rejection, actual tenant admission, no unattested publication, same-release restoration."
    );
}
