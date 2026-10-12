# Changelog

## [0.2.0] - 2026-10-12

- Make the native bridge installable as the `ic-crates` crates.io package alongside `ic-crates-protocol`, with package-local documentation and licensing.
- **Breaking:** rename the native package and executable from `ic-crates-publisher` to `ic-crates`; update build, install and launch commands. Registry and upload wire contracts are unchanged.
- Verify both package archives together and publish them in dependency order, checking each exact version independently before any upload.

## [0.1.1] - 2026-10-12

- Build from a fresh checkout with compatible released dependencies and explicit runtime identity checks.
- Package the protocol library independently with a README, license and verified minimum Rust version.
- Adopt reviewed Shared Tooling for setup, formatting, retained validation, CI and recoverable releases; keep package publication separate.
- Add a token-authenticated Cargo upload bridge with bounded archive validation, publish-to-index translation and authenticated IC transport.
- Preserve original permissions, archives and signed requests across interrupted work; dispatch the selected Blob Storage worker once and confirm publication through the catalog.
- Exercise unmodified Cargo publication, index polling, download and consumer compilation against a local substitute, including failure and replay checks.

## [0.1.0]

Initial development foundation; no release or package publication is recorded.

- Add local Canic/IcyDB/Blob/Memory composition and a dedicated blob component.
- Retain immutable registry configuration, exact upload intents and crate versions in IcyDB.
- Grant external uploader permissions from the registry tenant and inspect interrupted admissions.
- Require the configured verifier's raw-checksum attestation and a live blob descriptor before publication.
- Serve Cargo sparse-index metadata and blob redirects through HTTP update execution.
- Support publication replay, namespace collision checks, bounded catalogs and yanking without deleting archives.
