# IC Crates

A Rust package registry on the Internet Computer. Canic owns the managed app,
IcyDB stores its catalog, and IC Blob Storage owns archive storage and completion
verification. Default builds use released dependencies; sibling checkouts are
optional development inputs.

The foundation includes a controller-operated catalog, Cargo sparse-index and
blob download routes, and a token-authenticated native `cargo publish` bridge.
An unmodified Cargo client can publish, resolve, download and build through the
bridge in local fixture tests. Deployed IC/provider acceptance remains unqualified.
Earlier compatible-graph PocketIC evidence covers registry/blob installation,
tenant admission, controller rejection and restoration. The currently selected
IcyDB 0.272 graph awaits compatible Canic runtime adoption. The native
[Blob worker](docs/blob-worker.md) now has real SDK/Chromium local refusal evidence.
See [current evidence](docs/status/current.md).

## Upstream code reference

**[rust-lang/crates.io](https://github.com/rust-lang/crates.io)** is our primary
source reference for this port. Reuse and adapt its registry behavior, validation,
frontend components and tests where practical. Adapt persistence, lifecycle and
archive storage to IcyDB, Canic and IC Blob Storage.

Upstream is licensed under MIT or Apache-2.0. When porting code, record the source
commit and preserve the applicable copyright and license notices.
The first [documented code adaptations](crates/ic-crates-protocol/UPSTREAM.md)
cover dependency alias validation and transitive Cargo feature splitting.

## Components

| Path | Responsibility |
| --- | --- |
| `crates/ic-crates-protocol` | Cargo upload/index formats, bounded metadata validation and Candid contracts |
| `apps/ic_crates/registry` | IcyDB configuration/upload/version records, publication and HTTP reads |
| `apps/ic_crates/blob` | Dedicated Canic Blob Service shell using upstream endpoints and lifecycle |
| `apps/ic_crates/publisher` | Native Cargo HTTP bridge, archive validation, retained jobs and IC transport |
| `apps/ic_crates/tests` | Managed Wasm installation and registry/blob IC qualification |
| `apps/ic_crates/canic.toml` | Registry and target-initialized blob components |

Canic owns lifecycle and the memory allocation pool; IcyDB participates
synchronously after restoration. Each canister graph has one Memory and Timers
identity. The selected IcyDB 0.272 requires Timers 0.21 while Canic 0.111 still uses
Timers 0.17. This deliberate pending selection fails `make runtime-check`;
IC qualification awaits compatible Canic adoption.

## Publication

1. Install the managed app using matching Canic tooling. Initialize Blob Storage
   with its target-bound `ServiceInstallationInput`, then enroll the registry
   canister as a tenant using the upstream composition workflow.
2. A controller calls `registry_configure` once with the blob service, namespace,
   verifier, provider project and HTTPS provider origin. Exact replay succeeds;
   changing the immutable configuration is refused.
   IcyDB's asynchronous startup must finish first; a pending database diagnostic
   indicates that writes remain closed.
3. Run the [Cargo bridge](docs/cargo-publish.md) with explicit IC transport,
   controller identity, token, private spool and Blob Storage worker selections.
   Cargo sends its metadata and archive to `PUT /api/v1/crates/new`.
4. The bridge validates framing and archive structure, computes both raw SHA-256
   and the upstream provider manifest, and translates metadata to index format.
   It retains the original bytes and permission before calling
   `registry_admit_upload`. The registry saves its intent before upstream admission.
5. The selected worker prepares that exact manifest, sends archive bytes directly
   to the provider and obtains the configured verifier's completion attestation
   through Blob Storage's maintained tooling. SDK/browser state and recovery
   remain with that owner. A worker exit alone never proves completion.
6. The bridge submits `registry_publish`. The registry authenticates bounded
   attestation evidence, checks raw archive SHA-256 and a currently live reference,
   derives the download URL with the upstream encoder, then atomically commits.
   Cargo receives success only after catalog confirmation.

Retries bind the original metadata, scope, archive digest and permission. They
check catalog completion without dispatching admission or the worker again.
Missing evidence stops publication; it never authorizes another provider upload.
The original controller Candid endpoints also remain available for operator work.

Mutations currently require a controller. The bridge's private token represents
that operator's authority; publisher accounts, per-crate ownership, search and
browsing remain future work. Published references are retained: yanking changes
selection and keeps downloads available. No archive deletion/release endpoint exists.

## Cargo configuration

Use the bridge's public HTTPS origin for publishing and reads:

```toml
# .cargo/config.toml
[registry]
global-credential-providers = ["cargo:token"]

[registries.ic-crates]
index = "sparse+https://REGISTRY_BRIDGE_HOST/index/"
```

```sh
cargo login --registry ic-crates
cargo publish --registry ic-crates
```

A dependent package uses `example = { version = "0.1", registry = "ic-crates" }`.
The bridge advertises `api` and proxies catalog reads through authenticated replica
updates. Direct canister index reads remain available at
`sparse+https://REGISTRY_CANISTER_ID.icp0.io/index/`, with no upload API advertised.
Downloads redirect to Blob Storage. Cargo checksums use raw archive SHA-256,
independent of the provider tree root; dependency registry URLs are preserved.

Reads currently use HTTP query-to-update upgrade. Certified queries, ETags and
caching remain open. Catalog limits are 64 KiB per version, 128 versions and
1 MiB per crate index; archive admission is capped at 1 GiB. The bridge enforces
operator-selected lower limits and bounded decompression. Names are lowercase;
hyphen/underscore variants share one namespace. SemVer build metadata is refused.

## Development and release readiness

Use Rust 1.99.0; no sibling checkout is needed for standard checks.

```sh
make install-tools
make deps
make ci
make install-validation-tools # Linux
make lint check-secrets       # Linux
```

[Development setup](docs/development.md) documents supported hosts, minimum Rust
versions, local overrides and the reviewed Shared Tooling snapshot. Native Cargo
fixture tests do not qualify installed canisters or deployed provider behavior.
`make install-tools` prepares the matching locked Canic CLI and Testkit-owned
PocketIC setup. Install the Wasm target with `rustup target add wasm32-unknown-unknown`.
The full `make ci` includes the managed IC checks; they can also run independently:

```sh
make managed-build
make test-ic
```

`ic-crates` (the native CLI/library) and `ic-crates-protocol` are crates.io
candidates; canister packages remain private. After publication, install the
bridge and selected worker with `cargo install --locked ic-crates`, then run
`ic-crates /absolute/path/bridge.json`. The former command `ic-crates-publisher`
has been renamed. `make package-check` verifies both archives together without
uploading them. Versions, release commits, tags, pushes,
package publication and deployments are user-owned; readiness checks perform none
of those effects. See [user-operated release/publication](docs/publishing.md).

Use Canic's [blob composition guide](https://github.com/dragginzgame/canic/blob/main/docs/features/blob-storage/README.md)
and Blob Storage's [operator guide](https://github.com/dragginzgame/ic-blob-storage/blob/main/docs/operator-guide.md)
for installation, enrollment, transfer and verifier contracts.
