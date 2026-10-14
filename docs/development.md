# Development and qualification

The default Cargo workspace builds from a fresh checkout with released
dependencies; sibling repositories are not required. `Cargo.lock` fixes the
selected dependency graph. Canic 0.111 and IcyDB 0.270.3 both use Timers 0.17,
and all canister paths share Memory 0.35. `make runtime-check` verifies identities.
IcyDB 0.271 requires Timers 0.19, so it needs coordinated Canic adoption before
this canister graph can select it.

## Setup

Prepare Rust 1.99.0 for development, Rust 1.88.0 for the public protocol library,
and Rust 1.91.0 for the public `ic-crates` CLI/library and canister components. Cargo, Git, Bash, Perl, curl,
tar and xz are system prerequisites. macOS workflows use native Apple Silicon
or Intel tools. See the reviewed [shared setup guide](local-setup.md).

```sh
rustup toolchain install --profile minimal --component rustfmt --no-self-update
rustup toolchain install 1.88.0 --profile minimal --no-self-update
rustup toolchain install 1.91.0 --profile minimal --no-self-update
rustup target add wasm32-unknown-unknown
make install-tools
make deps
make tools-check
```

Common installers verify pinned downloads and reuse the shared executable store.
The local product extension selects the Canic CLI matching locked Canic and
the server CLI from locked IC Testkit through the common Rust installer.
Testkit owns PocketIC setup, compatibility checks, launch and cleanup; this
repository carries no server download pin or alternate launcher.
`make install-hooks` activates the reviewed formatting
hook only when the shared hook and its companions are committed/tracked.

## Commands

| Command | Scope |
| --- | --- |
| `make check`, `make test`, `make clippy`, `make docs-check` | Focused native Rust checks |
| `make fmt`, `make fmt-check` | Shared Cargo sorting and Rust formatting |
| `make ci` | Complete native and managed IC delivery gate with retained failure logs |
| `make managed-build` | Build all five roles through the matching Canic CLI |
| `make test-ic` | Install registry/blob artifacts and test actual IC admission/restoration |
| `make install-app-tools`, `make app-tools-check` | Prepare/check Canic and Testkit product tools |
| `make msrv-check` | Protocol at 1.88 and applications at 1.91 |
| `make package-check`, `make publish-dry-run` | Package and compile both public archives together |
| `make test-publish` | Test publication order, observation guards and interrupted uploads with local substitutes |
| `make install-validation-tools`, `make lint check-secrets` | Linux shell/workflow lint and Git history secret scan |
| `make shared-tooling-check`, `make check-pins` | Snapshot integrity and declared dependency policy |
| `make tasks`, `make cloc` | Shared maintenance catalog and Rust LOC |

Use relevant narrow checks for development and handoff under the
[validation cadence](../AGENTS.md#validation-policy).
`make ci` and Linux lint/security remain the full release checks.
No standard validation command publishes, releases or deploys.
Package checks use Cargo's normal registry handling to verify the CLI against
the newly staged protocol archive. They preserve `--locked`; explicitly offline
callers can fail at Cargo's staged-registry unpack step even with external caches
prepared. No check changes package versions or uses `--no-verify`.
The [CI workflow](../.github/workflows/ci.yml) qualifies Ubuntu 24.04, macOS 15
Apple Silicon and macOS 15 Intel. Configuring CI is not evidence that it has run.
The native bridge's unmodified Cargo fixture runs in the standard tests.
Managed Wasm installation and registry/blob admission/restoration run in the
standard delivery gate. Deployed gateway behavior and real provider acceptance
remain separate. See the [bridge contract](cargo-publish.md).

## Managed IC qualification

`make managed-build` asks matching Canic tooling to build Root, Coordinator,
Store, Registry and Blob artifacts with the Fast profile. Each attempt retains
its configuration, JSON result and diagnostics under `target/managed-builds`.
Only a completed all-role result becomes `target/managed-build.json`.

`make test-ic` uses Testkit's managed server command and Canic's published
component-group fixture. The fixture supplies a synthetic Root and installs
the actual Registry and Blob Wasm with canonical target-bound initialization.
The test validates the sealed manifest, artifact union, raw Wasm hashes and
Candid exports before installation. It proves controller rejection, immutable
configuration, actual registry-tenant upload admission and exact replay,
conflicting-intent rejection, refusal of unattested publication and same-release
restoration of both canisters. Testkit's bounded polling helper handles IcyDB's
separate startup condition, matching the selected owner's typed pending code.
Other errors fail immediately; upload and provider operations are not retried.

Server logs remain under `target/managed-tests`; CI retains these, build
attempts and a shared-helper archive of the managed Wasm directory on failure.
This fixture does not qualify production Fleet Root
provisioning, cross-release upgrades, an HTTP gateway, browser transfer, real
completion verification or paid storage. The standard native Cargo fixture
continues to cover publication-to-consumption against a local substitute.

The native Cargo fixture publishes a dependency and a consumer crate, then builds
through the registry with an underscore-prefixed alias and transitive
default/namespaced/weak features. The protocol's
[upstream attribution](../crates/ic-crates-protocol/UPSTREAM.md) records the source
revision, adaptations and included MIT notice. Regression evidence before the
fix remains in `target/upstream-cargo-regression-before.log`.

## Local sibling work

Local development can opt into Cargo's CLI patch configuration, for example:

```sh
cargo --config 'patch.crates-io.ic-blob-storage-contracts.path="../ic-blob-storage/crates/ic-blob-storage-contracts"' check -p ic-crates-protocol
```

Use a disposable clone or a separately retained development lockfile for a
different sibling graph. A local override is not publication evidence. Select
one Canic source, one Memory source and one Timers runtime across each canister
graph; do not combine framework versions or bypass compiler warnings.

## Shared Tooling

The [snapshot manifest](../.shared-tooling.snapshot) records Shared Tooling 0.9.5
at `b1630b89482cc6071fc928a86962d79c9c11fdb7`. The exporter selected a clean copy
of that commit, preserving the sibling's unrelated work. Vendored files remain
unchanged; local commands and release metadata belong to this repository.

Review the [maintenance catalog](../tasks/README.md) for dependency security,
toolchain/MSRV checks, snapshot drift, README freshness and unused dependencies.
These procedures are available; no recurring schedule is enabled automatically.
