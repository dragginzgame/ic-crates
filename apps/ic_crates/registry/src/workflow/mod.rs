//! Authenticated blob observations precede the catalog's single atomic commit.

use std::num::{NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize};

use ic_blob_storage::ops::service::reads::download::client::ReplicatedDownloadClient;
use ic_blob_storage::ops::service::uploads::admission::client::ReplicatedUploadAdmissionClient;
use ic_blob_storage_contracts::dto::upload::admission::{
    UploadAdmissionMutation, UploadAdmissionRequest, UploadAdmissionResponse,
};
use ic_blob_storage_contracts::{
    download::{reply::DownloadReplyLimits, scope::CaffeineDownloadScope},
    dto::{
        download::DownloadRequest,
        upload::completion::{UploadAttestationLookup, UploadAttestationResponse},
    },
    identity::ProviderRootHash,
    protocol::UPLOAD_ATTESTATION_METHOD,
    provider::download::request_target,
    upload::completion::{CompletionAuthority, reply},
};
use ic_cdk::call::Call;
use ic_crates_protocol::{IndexEntry, PublishRequest, RegistryError, checksum_hex};

use crate::ops;

fn admission_client(
    permission: UploadAdmissionRequest,
) -> Result<ReplicatedUploadAdmissionClient, RegistryError> {
    let config = ops::configuration()?;
    let upload = permission.upload;
    if upload.service != config.blob_service
        || upload.namespace != config.namespace
        || upload.tenant != ic_cdk::api::canister_self()
    {
        return Err(RegistryError::BlobBinding);
    }
    if upload.bytes > 1024 * 1024 * 1024 {
        return Err(RegistryError::Limit);
    }
    ic_blob_storage_contracts::upload::admission::parse_binding(config.blob_service, permission)
        .map_err(|_| RegistryError::BlobBinding)?;
    ReplicatedUploadAdmissionClient::new(
        upload.tenant,
        upload.service,
        NonZeroU32::new(30).ok_or(RegistryError::Internal)?,
    )
    .map_err(|_| RegistryError::BlobBinding)
}

pub(crate) async fn admit_upload(
    permission: UploadAdmissionRequest,
) -> Result<UploadAdmissionMutation, RegistryError> {
    let client = admission_client(permission)?;
    let intent = candid::encode_one(permission).map_err(|_| RegistryError::Internal)?;
    ops::retain_upload(permission.upload.upload, intent)?;
    client
        .admit(permission, nonzero(8192)?)
        .await
        .map_err(|_| RegistryError::BlobUnavailable)
}

pub(crate) async fn inspect_upload(
    permission: UploadAdmissionRequest,
) -> Result<UploadAdmissionResponse, RegistryError> {
    let client = admission_client(permission)?;
    let intent = candid::encode_one(permission).map_err(|_| RegistryError::Internal)?;
    ops::check_upload(permission.upload.upload, &intent)?;
    client
        .inspect(permission, nonzero(8192)?)
        .await
        .map_err(|_| RegistryError::BlobUnavailable)
}

pub(crate) async fn publish(request: PublishRequest) -> Result<(), RegistryError> {
    let entry = IndexEntry::parse(&request.index_json)?;
    if entry.vers.len() > 128 {
        return Err(RegistryError::InvalidVersion);
    }
    let config = ops::configuration()?;
    let upload = request.permission.upload;
    let permission = candid::encode_one(request.permission).map_err(|_| RegistryError::Internal)?;
    ops::check_upload(upload.upload, &permission)?;
    if upload.service != config.blob_service
        || upload.namespace != config.namespace
        || upload.tenant != ic_cdk::api::canister_self()
    {
        return Err(RegistryError::BlobBinding);
    }
    // Fail name conflicts before remote work; the final transaction checks version conflicts.
    let _ = ops::versions(&entry.name)?;
    let namespace =
        NonZeroU128::new(config.namespace).ok_or(RegistryError::InvalidConfiguration)?;
    let authority =
        CompletionAuthority::new(config.blob_service, namespace, config.completion_verifier)
            .map_err(|_| RegistryError::InvalidConfiguration)?;
    let arguments = reply::inspection_request(authority, request.permission)
        .map_err(|_| RegistryError::BlobBinding)?;
    let response = Call::bounded_wait(config.blob_service, UPLOAD_ATTESTATION_METHOD)
        .take_raw_args(arguments)
        .await
        .map_err(|_| RegistryError::BlobUnavailable)?;
    let attestation = reply::inspection(
        authority,
        request.permission,
        response.as_ref(),
        nonzero(8192)?,
    )
    .map_err(|_| RegistryError::BlobUnavailable)?;
    check_archive_attestation(&attestation, &entry.cksum)?;

    // A historical attestation cannot substitute for a currently live reference.
    let scope =
        CaffeineDownloadScope::new(config.blob_service, namespace, &config.provider_project)
            .map_err(|_| RegistryError::InvalidConfiguration)?;
    let download = DownloadRequest {
        service: upload.service,
        tenant: upload.tenant,
        namespace: upload.namespace,
        root: upload.root,
        object: upload.object,
        incarnation: upload.incarnation,
        reference: upload.first_reference,
    };
    let client = ReplicatedDownloadClient::new(
        upload.tenant,
        config.blob_service,
        NonZeroU32::new(30).ok_or(RegistryError::Internal)?,
    )
    .map_err(|_| RegistryError::BlobBinding)?;
    let descriptor = client
        .fetch(
            download,
            &scope,
            DownloadReplyLimits {
                max_reply_bytes: nonzero(16 * 1024)?,
                max_content_bytes: NonZeroU64::new(1024 * 1024 * 1024)
                    .ok_or(RegistryError::Internal)?,
                max_headers: nonzero(32)?,
                max_header_bytes: nonzero(8192)?,
            },
        )
        .await
        .map_err(|_| RegistryError::BlobUnavailable)?;
    if descriptor.bytes != upload.bytes {
        return Err(RegistryError::BlobBinding);
    }
    let root = ProviderRootHash::try_from(upload.root.as_slice())
        .map_err(|_| RegistryError::BlobBinding)?;
    let url = format!("{}{}", config.provider_origin, request_target(&scope, root));
    ops::commit(&entry, permission, url)
}

fn nonzero(value: usize) -> Result<NonZeroUsize, RegistryError> {
    NonZeroUsize::new(value).ok_or(RegistryError::Internal)
}

/// Apply only after the upstream bounded decoder authenticates the full permission/verifier binding.
fn check_archive_attestation(
    attestation: &UploadAttestationResponse,
    checksum: &str,
) -> Result<(), RegistryError> {
    if attestation.fenced {
        return Err(RegistryError::BlobUnavailable);
    }
    let UploadAttestationLookup::Found(receipt) = attestation.attestation else {
        return Err(RegistryError::BlobUnavailable);
    };
    if checksum_hex(&receipt.request.content_digest) != checksum {
        return Err(RegistryError::ChecksumMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use ic_blob_storage_contracts::dto::{
        reference::ReferenceUpload,
        upload::completion::{UploadAttestationReceipt, UploadAttestationRequest},
    };

    #[test]
    fn publication_uses_the_attested_raw_checksum_and_refuses_missing_or_fenced_evidence() {
        let permission = UploadAdmissionRequest {
            upload: ReferenceUpload {
                service: Principal::from_slice(&[1]),
                tenant: Principal::from_slice(&[2]),
                namespace: 1,
                upload: 1,
                object: 2,
                incarnation: 1,
                first_reference: 3,
                root: [0xab; 32],
                bytes: 10,
            },
            uploader: Principal::from_slice(&[3]),
            expires_at_ns: 100,
        };
        let mut observation = UploadAttestationResponse {
            permission,
            fenced: false,
            attestation: UploadAttestationLookup::Found(UploadAttestationReceipt {
                request: UploadAttestationRequest {
                    permission,
                    content_digest: [0xcd; 32],
                    observed_at_ns: 50,
                },
                verifier: Principal::from_slice(&[4]),
                accepted_at_ns: 51,
            }),
        };
        assert_eq!(
            check_archive_attestation(&observation, &"cd".repeat(32)),
            Ok(())
        );
        assert_eq!(
            check_archive_attestation(&observation, &"ab".repeat(32)),
            Err(RegistryError::ChecksumMismatch)
        );
        observation.fenced = true;
        assert_eq!(
            check_archive_attestation(&observation, &"cd".repeat(32)),
            Err(RegistryError::BlobUnavailable)
        );
        observation.fenced = false;
        observation.attestation = UploadAttestationLookup::Absent;
        assert_eq!(
            check_archive_attestation(&observation, &"cd".repeat(32)),
            Err(RegistryError::BlobUnavailable)
        );
    }
}
