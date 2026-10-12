//! Bounded catalog reads and atomic, synchronous IcyDB transactions.

use crate::design::{
    ConfigurationRecord, ConfigurationRecordInsert, UploadRecord, UploadRecordInsert,
    VersionRecord, VersionRecordInsert,
};
use ic_crates_protocol::{IndexEntry, RegistryConfiguration, RegistryError, canonical_name};
use icydb::{
    db::{StructuralMutation, StructuralPatch, TypedOperationError, WriteCell, query::asc},
    traits::EntitySource as _,
    types::Id,
    value::InputValue,
};

pub(crate) const MAX_VERSIONS: usize = 128;
pub(crate) const MAX_CRATE_INDEX_BYTES: usize = 1024 * 1024;

pub(crate) fn database_error(error: icydb::Error) -> RegistryError {
    RegistryError::Database {
        code: u32::from(error.code().raw()),
    }
}

fn operation_error(error: TypedOperationError) -> RegistryError {
    match error {
        TypedOperationError::Database(error) => database_error(error),
        TypedOperationError::Adapter(_) => RegistryError::Internal,
    }
}

/// Preserve the original exact permission before any remote admission effect.
pub(crate) fn retain_upload(id: u128, permission: Vec<u8>) -> Result<(), RegistryError> {
    let db = crate::db().map_err(database_error)?;
    if let Some(retained) = db
        .get::<UploadRecord>(Id::from_key(id))
        .map_err(operation_error)?
    {
        return if retained.permission.as_bytes() == permission {
            Ok(())
        } else {
            Err(RegistryError::UploadConflict)
        };
    }
    let mut batch = db.trusted_typed_write_batch();
    batch
        .push(UploadRecordInsert {
            id: WriteCell::Value(Id::from_key(id)),
            permission: WriteCell::Value(permission.into()),
        })
        .map_err(operation_error)?;
    batch.execute().map_err(operation_error)?;
    Ok(())
}

pub(crate) fn check_upload(id: u128, permission: &[u8]) -> Result<(), RegistryError> {
    let retained = crate::db()
        .map_err(database_error)?
        .get::<UploadRecord>(Id::from_key(id))
        .map_err(operation_error)?
        .ok_or(RegistryError::NotFound)?;
    if retained.permission.as_bytes() != permission {
        return Err(RegistryError::UploadConflict);
    }
    Ok(())
}

pub(crate) fn configuration() -> Result<RegistryConfiguration, RegistryError> {
    let row = crate::db()
        .map_err(database_error)?
        .get::<ConfigurationRecord>(Id::from_key(1_u64))
        .map_err(operation_error)?
        .ok_or(RegistryError::NotConfigured)?;
    Ok(RegistryConfiguration {
        blob_service: row.blob_service.into(),
        completion_verifier: row.completion_verifier.into(),
        namespace: row.namespace,
        provider_project: row.provider_project,
        provider_origin: row.provider_origin,
    })
}

pub(crate) fn configure(config: &RegistryConfiguration) -> Result<(), RegistryError> {
    config.validate()?;
    match configuration() {
        Ok(retained) if &retained == config => return Ok(()),
        Ok(_) => return Err(RegistryError::ConfigurationConflict),
        Err(RegistryError::NotConfigured) => {}
        Err(error) => return Err(error),
    }
    let db = crate::db().map_err(database_error)?;
    let mut batch = db.trusted_typed_write_batch();
    batch
        .push(ConfigurationRecordInsert {
            id: WriteCell::Value(Id::from_key(1_u64)),
            blob_service: WriteCell::Value(config.blob_service.into()),
            completion_verifier: WriteCell::Value(config.completion_verifier.into()),
            namespace: WriteCell::Value(config.namespace),
            provider_project: WriteCell::Value(config.provider_project.clone()),
            provider_origin: WriteCell::Value(config.provider_origin.clone()),
        })
        .map_err(operation_error)?;
    batch.execute().map_err(operation_error)?;
    Ok(())
}

/// Bounded by both lifetime version count and aggregate index bytes at publication.
pub(crate) fn versions(name: &str) -> Result<Vec<VersionRecord>, RegistryError> {
    let page = crate::db()
        .map_err(database_error)?
        .query::<VersionRecord>()
        .map_err(operation_error)?
        .filter(VersionRecord::CRATE_KEY.eq(canonical_name(name)?))
        .order_by(asc(VersionRecord::VERSION))
        .limit((MAX_VERSIONS + 1) as u32)
        .execute_live_page(None)
        .map_err(operation_error)?;
    if page.continuation.is_some() || page.rows.len() > MAX_VERSIONS {
        return Err(RegistryError::Limit);
    }
    // Namespace collisions are rejected; a Cargo URL must use the retained spelling.
    if page.rows.iter().any(|row| row.name != name) {
        return Err(RegistryError::NameConflict);
    }
    Ok(page.rows)
}

pub(crate) fn version(name: &str, version: &str) -> Result<VersionRecord, RegistryError> {
    versions(name)?
        .into_iter()
        .find(|row| row.version == version)
        .ok_or(RegistryError::NotFound)
}

/// Re-read after external observations. No await can occur between these checks and commit.
pub(crate) fn commit(
    entry: &IndexEntry,
    permission: Vec<u8>,
    url: String,
) -> Result<(), RegistryError> {
    let line = entry.line()?;
    let rows = versions(&entry.name)?;
    if let Some(existing) = rows.iter().find(|row| row.version == entry.vers) {
        return if existing.index_json == line
            && existing.permission.as_bytes() == permission
            && existing.download_url == url
        {
            Ok(())
        } else {
            Err(RegistryError::VersionConflict)
        };
    }
    if rows.len() >= MAX_VERSIONS
        || rows.iter().map(|r| r.index_json.len()).sum::<usize>() + line.len()
            > MAX_CRATE_INDEX_BYTES
    {
        return Err(RegistryError::Limit);
    }
    let db = crate::db().map_err(database_error)?;
    let mut batch = db.trusted_typed_write_batch();
    batch
        .push(VersionRecordInsert {
            crate_key: WriteCell::Value(canonical_name(&entry.name)?),
            name: WriteCell::Value(entry.name.clone()),
            version: WriteCell::Value(entry.vers.clone()),
            index_json: WriteCell::Value(line),
            permission: WriteCell::Value(permission.into()),
            download_url: WriteCell::Value(url),
            yanked: WriteCell::Value(false),
        })
        .map_err(operation_error)?;
    batch.execute().map_err(operation_error)?;
    Ok(())
}

pub(crate) fn yank(name: &str, version: &str, yanked: bool) -> Result<(), RegistryError> {
    let row = self::version(name, version)?;
    if row.yanked == yanked {
        return Ok(());
    }
    crate::db()
        .map_err(database_error)?
        .execute_trusted_structural_mutation(StructuralMutation::Update {
            entity: VersionRecord::ENTITY.to_owned(),
            key: InputValue::ulid(row.id),
            patch: StructuralPatch::new().field(
                VersionRecord::YANKED.as_str(),
                WriteCell::Value(InputValue::boolean(yanked)),
            ),
        })
        .map_err(database_error)?;
    Ok(())
}

pub(crate) fn index(name: &str) -> Result<String, RegistryError> {
    let rows = versions(name)?;
    if rows.is_empty() {
        return Err(RegistryError::NotFound);
    }
    let mut result = String::new();
    for row in rows {
        let mut entry: IndexEntry =
            serde_json::from_str(&row.index_json).map_err(|_| RegistryError::Internal)?;
        entry.yanked = row.yanked;
        result.push_str(&entry.line()?);
    }
    if result.len() > MAX_CRATE_INDEX_BYTES {
        return Err(RegistryError::Limit);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use icydb::db::with_request_execution;

    #[test]
    fn catalog_publication_is_immutable_replayable_and_yanking_preserves_downloads() {
        crate::__icydb_generated::__drive_native_database_for_tests().expect("initialize catalog");
        with_request_execution(|| {
            let config = RegistryConfiguration {
                blob_service: Principal::from_slice(&[1]),
                completion_verifier: Principal::from_slice(&[2]),
                namespace: 1,
                provider_project: "crates".into(),
                provider_origin: "https://storage.example".into(),
            };
            assert_eq!(configuration(), Err(RegistryError::NotConfigured));
            configure(&config).unwrap();
            configure(&config).unwrap();
            let mut foreign = config.clone();
            foreign.namespace = 2;
            assert_eq!(
                configure(&foreign),
                Err(RegistryError::ConfigurationConflict)
            );
            assert_eq!(configuration().unwrap(), config);

            retain_upload(1, vec![1, 2, 3]).unwrap();
            retain_upload(1, vec![1, 2, 3]).unwrap();
            assert_eq!(
                retain_upload(1, vec![4]),
                Err(RegistryError::UploadConflict)
            );
            check_upload(1, &[1, 2, 3]).unwrap();
            assert_eq!(check_upload(1, &[4]), Err(RegistryError::UploadConflict));
            assert_eq!(check_upload(2, &[1, 2, 3]), Err(RegistryError::NotFound));

            let json = format!(
                r#"{{"name":"demo_crate","vers":"0.1.0","deps":[],"cksum":"{}","features":{{}},"yanked":false}}"#,
                "ab".repeat(32)
            );
            let entry = IndexEntry::parse(&json).unwrap();
            let url = "https://storage.example/v1/blob/?blob_hash=sha256:123".to_owned();
            commit(&entry, vec![1, 2, 3], url.clone()).unwrap();
            commit(&entry, vec![1, 2, 3], url.clone()).unwrap();
            assert_eq!(versions("demo_crate").unwrap().len(), 1);
            assert_eq!(index("demo_crate").unwrap(), entry.line().unwrap());
            assert_eq!(
                commit(&entry, vec![4], url.clone()),
                Err(RegistryError::VersionConflict)
            );
            let mut changed = entry.clone();
            changed.cksum = "cd".repeat(32);
            assert_eq!(
                commit(&changed, vec![1, 2, 3], url.clone()),
                Err(RegistryError::VersionConflict)
            );
            changed = entry.clone();
            changed.name = "demo-crate".into();
            assert_eq!(
                commit(&changed, vec![1, 2, 3], url.clone()),
                Err(RegistryError::NameConflict)
            );

            yank("demo_crate", "0.1.0", true).unwrap();
            let yanked: IndexEntry =
                serde_json::from_str(index("demo_crate").unwrap().trim()).unwrap();
            assert!(yanked.yanked);
            commit(&entry, vec![1, 2, 3], url.clone()).unwrap();
            assert!(version("demo_crate", "0.1.0").unwrap().yanked);
            let read = |method: &str, path: &str| {
                crate::http::serve(&crate::http::HttpRequest {
                    method: method.into(),
                    url: path.into(),
                    headers: Vec::new(),
                    body: Vec::new(),
                    certificate_version: None,
                })
            };
            let download = read("GET", "/api/v1/crates/demo_crate/0.1.0/download");
            assert_eq!(download.status_code, 302);
            assert!(download.headers.contains(&("Location".into(), url)));
            assert_eq!(read("GET", "/index/de/mo/demo_crate").status_code, 200);
            assert_eq!(read("GET", "/index/xx/xx/demo_crate").status_code, 404);
            assert_eq!(read("GET", "/index/de/mo/demo-crate").status_code, 404);
            assert!(read("HEAD", "/index/de/mo/demo_crate").body.is_empty());
            assert_eq!(read("PUT", "/api/v1/crates/new").status_code, 405);
            yank("demo_crate", "0.1.0", false).unwrap();
            assert!(!version("demo_crate", "0.1.0").unwrap().yanked);
        });
    }
}
