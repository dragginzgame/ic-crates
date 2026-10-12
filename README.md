# IC Crates

A Rust package registry on the Internet Computer, built with local Canic,
IcyDB, IC Blob Storage and IC Memory checkouts. Crate archives go directly to
the blob provider; the registry stores metadata and publication evidence.

This is the first backend foundation. It has a controller-operated publication
API and Cargo-compatible sparse-index/download routes. It has not been deployed
or exercised against a real provider, and normal `cargo publish` is not connected
yet. [Current validation and next work](docs/status/current.md).

## Components

| Path | Responsibility |
| --- | --- |
| `crates/ic-crates-protocol` | Cargo index format, bounded metadata validation and shared registry DTOs |
| `apps/ic_crates/registry` | IcyDB configuration/upload/version records, publication workflow and HTTP reads |
| `apps/ic_crates/blob` | Dedicated Canic Blob Service shell, using upstream endpoints and lifecycle |
| `apps/ic_crates/canic.toml` | One registry and one target-initialized blob component |

Canic owns the registry lifecycle and the single memory allocation pool. IcyDB
participates synchronously after Canic restoration and owns its journaled catalog.
All production dependencies share one Memory identity and one Timers identity.
The blob shell has its own canister identity and memory pool.

The root Cargo catalog uses sibling paths for Canic, IcyDB, Blob Storage and
Memory. Timers and Metrics currently use their published versions: their sibling
development APIs differ from those consumed by local Canic. `Cargo.lock` records
the selected graph; path dependencies remain tied to the contents of the local
checkouts. No sibling source files are owned by this repository.

## Publication flow

1. Build/install the managed Canic app. Bind the blob component's target-specific
   `ServiceInstallationInput` using Canic's existing initialization mechanism.
   Its service identity, namespace, provider project and completion verifier must
   match the registry's configuration. Enroll the registry canister as a blob tenant.
2. A registry controller calls `registry_configure` once with the blob service,
   verifier, namespace, provider project and HTTPS provider origin. Exact replay
   succeeds; changing this configuration is refused.
3. Prepare a `.crate` archive and its manifest using Blob Storage's existing tools.
   The upload permission names the registry as tenant and an external signer as
   uploader. Call `registry_admit_upload`: the registry saves the full intent in
   IcyDB before granting permission through the blob service. Use
   `registry_upload_admission` to inspect an uncertain outcome with the same intent.
4. The uploader prepares the manifest directly with the blob service and sends
   the bytes directly to Caffeine. The configured verifier checks the content and
   records its attestation through the upstream completion flow.
5. Call `registry_publish` with the original permission and an **index-format** JSON
   entry. The registry authenticates the configured blob service, validates its
   bounded attestation reply, compares the verified raw archive SHA-256 with Cargo's
   checksum, and obtains a current live-reference descriptor. It constructs the
   download URL from the configured origin and upstream provider URL encoder.
6. The registry commits the version atomically. Exact replay succeeds; changing
   metadata, upload permission or download URL for that version is refused.

Cargo's checksum is raw archive SHA-256, in plain lowercase hex. It is **not** the
provider's tree root. Dependencies from crates.io require the explicit index URL
`https://github.com/rust-lang/crates.io-index`; an index dependency's `registry: null`
means this registry. Publish-API metadata has a different dependency field layout
and must eventually be translated by the upload bridge.

All registry mutations currently require a controller. The controller is responsible
for checking that archive contents and Cargo metadata agree; archive parsing and
package validation belong to the next host-side publish bridge. There are no public
publisher accounts, tokens, ownership transfers or search API yet.

Published blob references are retained. Yanking changes dependency selection and
preserves the archive URL. There is no registry release/delete endpoint. A descriptor
is an observation rather than a permanent availability guarantee; provider retention
and consumer reference coordination still need integration qualification.

## Cargo reads

After managed installation and publication, configure an alternative registry:

```toml
# .cargo/config.toml
[registries.ic-crates]
index = "sparse+https://REGISTRY_CANISTER_ID.icp0.io/index/"
```

```toml
# Cargo.toml
[dependencies]
example = { version = "0.1", registry = "ic-crates" }
```

The registry serves `/index/config.json`, Cargo's sharded index paths, and
`/api/v1/crates/{crate}/{version}/download`. The download route redirects to blob
storage. `config.json` deliberately omits `api` until the Cargo publish API exists.

For this foundation, `http_request` asks the gateway to upgrade reads to update
execution, and `http_request_update` reads IcyDB. This avoids serving an uncertified
query index but makes reads use consensus and consume cycles. Certified queries,
ETags, caching, streaming and gateway/Cargo integration proof are still required.
The initial limits are 64 KiB metadata per version, 128 versions and 1 MiB of index
data per crate, and 1 GiB per archive. Names are lowercase; hyphen/underscore
variants share one namespace. Versions with semver build metadata are refused.

## Local development

Keep sibling repositories beside this checkout and use Rust 1.99.0.

```sh
make fmt
make test-protocol
make check
make test-catalog
```

The current Canic development checkout is undergoing Auth integration work;
normal Canic-dependent checks stop on five upstream dead-code findings.
To inspect this foundation while that work continues, the recorded diagnostic
checks used `RUSTFLAGS='--cap-lints=warn'`. That override is not persisted in the
build configuration and does not establish strict upstream validation. Consult
the current handoff before treating these passes as evidence for later sibling contents.

Managed Wasm builds must use the matching **0.111** Canic CLI rather than direct
Cargo Wasm builds. The discovered local CLI was 0.110.54 and was not used to build
these components. Once the matching CLI and upstream graph are ready:

```sh
canic build ic_crates --workspace . --config apps/ic_crates/canic.toml \
  --icp-root apps/ic_crates --profile fast --json
```

Building does not install the app or configure/fund a provider. Use Canic's
[blob composition guide](https://github.com/dragginzgame/canic/blob/main/docs/features/blob-storage/README.md) and Blob
Storage's [operator guide](https://github.com/dragginzgame/ic-blob-storage/blob/main/docs/operator-guide.md) for their
existing initialization, enrollment, upload and verifier contracts.
