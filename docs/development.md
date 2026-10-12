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
make install-tools
make deps
make tools-check
```

Common installers verify pinned downloads and reuse the shared executable store.
They do not install a Canic product CLI or PocketIC: those owners retain their
explicit setup contracts. `make install-hooks` activates the reviewed formatting
hook only when the shared hook and its companions are committed/tracked.

## Commands

| Command | Scope |
| --- | --- |
| `make check`, `make test`, `make clippy`, `make docs-check` | Focused native Rust checks |
| `make fmt`, `make fmt-check` | Shared Cargo sorting and Rust formatting |
| `make ci` | Complete native delivery gate with retained failure logs |
| `make msrv-check` | Protocol at 1.88 and applications at 1.91 |
| `make package-check`, `make publish-dry-run` | Package and compile both public archives together |
| `make test-publish` | Test publication order, observation guards and interrupted uploads with local substitutes |
| `make install-validation-tools`, `make lint check-secrets` | Linux shell/workflow lint and Git history secret scan |
| `make shared-tooling-check`, `make check-pins` | Snapshot integrity and declared dependency policy |
| `make tasks`, `make cloc` | Shared maintenance catalog and Rust LOC |

Run relevant narrow checks while coding; complete `make ci` and Linux lint/security
before delivery. No standard validation command publishes, releases or deploys.
Package checks use Cargo's normal registry handling to verify the CLI against
the newly staged protocol archive. They preserve `--locked`; explicitly offline
callers can fail at Cargo's staged-registry unpack step even with external caches
prepared. No check changes package versions or uses `--no-verify`.
The [CI workflow](../.github/workflows/ci.yml) qualifies Ubuntu 24.04, macOS 15
Apple Silicon and macOS 15 Intel. Configuring CI is not evidence that it has run.
The native bridge's unmodified Cargo fixture runs in the standard tests.
Wasm installation, deployed gateway behavior and real provider acceptance remain
separate application qualification. See the [bridge contract](cargo-publish.md).

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
