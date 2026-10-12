# Changelog

## 0.1.0 (draft)

- Add local Canic/IcyDB/Blob/Memory composition and a dedicated blob component.
- Retain immutable registry configuration, exact upload intents and crate versions in IcyDB.
- Grant external uploader permissions from the registry tenant and inspect interrupted admissions.
- Require the configured verifier's raw-checksum attestation and a live blob descriptor before publication.
- Serve Cargo sparse-index metadata and blob redirects through HTTP update execution.
- Support publication replay, namespace collision checks, bounded catalogs and yanking without deleting archives.
