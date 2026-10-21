# Changelog

## [0.3.0]

- Run Clippy before native compilation and tests; a lint failure stops later validation and managed builds.

- Prepare locked workspace dependencies before selecting application tools, and stop on failed offline graph selection ([#9 — Prepare the selected workspace cache before offline app-tool selection](https://github.com/dragginzgame/ic-crates/issues/9)).
- Add `ic-crates-blob-worker` and an offline preflight command composing the maintained Blob CLI, SDK/browser journal and verifier for an already-admitted registry tenant upload.
- Accept selected worker executables up to 256 MiB, including the supported debug build.
- Bind explicit tools, signers and budgets to retained original jobs; refuse redispatch and leave final publication confirmation with the registry.
- **Breaking:** hash new archive manifests with `Content-Type: application/gzip` to match the upstream SDK. Existing jobs require their original worker and journals; do not rewrite admitted manifests or repeat uncertain transfers.
- Exercise a clean released Blob tool selection, real SDK manifest agreement across multiple leaves, Chromium refusal and retained signed uncertainty against a local substitute.
- Select managed server tooling from the test harness's resolved Testkit dependency when upstream SDKs retain another Testkit version. Retain the selected IcyDB 0.273 / Blob 0.23 / Memory 0.36 graph pending compatible Canic adoption; report conflicting runtime identities in validation failures.

## [0.2.1] - 2026-10-12

- Port crates.io dependency alias and transitive feature-splitting behavior, allowing underscore-prefixed aliases and keeping extended feature chains together in the Cargo index.
- Qualify renamed optional dependencies and default/weak feature activation by publishing and consuming two crates with unmodified Cargo.
- Fix managed Canic builds by declaring the explicit feature lists required for role-contract admission.
- Add managed registry/blob IC tests for controller rejection, real tenant upload admission, unattested-publication refusal and same-release restoration.
- Extend the delivery gate with matching Canic artifacts and Testkit-owned PocketIC setup, bounded startup polling and retained build/server evidence.

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
