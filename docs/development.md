# Development and qualification

Default dependency declarations select published crates and the root lockfile;
sibling repositories are not build inputs. The latest maintained selections are
Blob Client/Contracts 0.48.0, Host FS/Process and Testkit 0.68.5, Memory 0.51.3 and IcyDB
0.284.1. Canic remains at its latest published 0.111.0 family.

This prepared graph is **not buildable yet**: Canic still owns Memory 0.35, while
new components use Memory 0.51. `make runtime-check` rejects that split. Blob's
service contracts now use IC Wire and no longer implement Candid for the permission
embedded in our public protocol. The focused native check fails there too.
Canic must provide the compatible component/client boundary before qualification;
do not add a DTO mirror, old/new codec fallback or independent bootstrap here.
Earlier passing evidence does not qualify this selected graph.

## Setup

Prepare Rust 1.99.1 for development and Rust 1.91.0 for the public packages and
canister components. Blob contracts require the protocol's Rust 1.91 floor.
Cargo, Git, Bash, Perl, curl,
tar and xz are system prerequisites. macOS workflows use native Apple Silicon
or Intel tools. See the reviewed [shared setup guide](local-setup.md).

```sh
rustup toolchain install --profile minimal --component rustfmt --no-self-update
rustup toolchain install 1.91.0 --profile minimal --no-self-update
rustup target add wasm32-unknown-unknown
make install-tools
make deps
make tools-check
```

Common installers verify pinned downloads and reuse the shared executable store.
The shared default set is host tools plus Cargo sort/derive sort. This product
explicitly selects `didc ic-wasm icp quill wasm-opt` through `IC_TOOLS` and the
ordered local setup/check target lists. CI's compact failure evidence uses that
same selection. ICP CLI now uses the snapshot's reviewed 1.7.0 catalog entry.

The local product setup selects matching Canic and the harness's resolved direct
Testkit version. Testkit installation and admission both request `--features cli`;
Testkit alone owns PocketIC setup and lifecycle. Canic 0.111's legacy declaration
extractor is now an explicit product tool at 0.1.6. The IC Build release editor
is selected at 0.68.5 with `cli`, debug profile and a 4 MiB document allowance.
[Product pins](../ci/product-tools.env) stay separate from shared default catalogs.
The release adapter obtains the admitted editor path automatically; operators
need not export it. There is one main snapshot and no auxiliary installer copy.
`make install-hooks` activates the reviewed formatting
hook only when the shared hook and its companions are committed/tracked.

## Commands

| Command | Scope |
| --- | --- |
| `make check`, `make test`, `make clippy`, `make docs-check` | Focused native Rust checks |
| `make fmt`, `make fmt-check` | Shared Cargo sorting and Rust formatting |
| `make ci` | Complete native and managed IC delivery gate with retained failure logs |
| `make test-blob-worker` | Opt-in real SDK/Chromium local refusal test with explicit tool configuration ([setup](blob-worker.md)) |
| `make managed-build` | Build all five roles through the matching Canic CLI |
| `make test-ic` | Install registry/blob artifacts and test actual IC admission/restoration |
| `make install-app-tools`, `make app-tools-check` | Prepare/check Canic and Testkit product tools |
| `make msrv-check` | All packages at the required 1.91 floor (source qualification pending) |
| `make package-check`, `make publish-dry-run` | Package and compile both public archives together |
| `make test-publish` | Test publication order, observation guards and interrupted uploads with local substitutes |
| `make install-validation-tools`, `make lint check-secrets` | Linux shell/workflow lint and Git history secret scan |
| `make shared-tooling-check`, `make check-pins` | Snapshot integrity and declared dependency policy |
| `make deployment-deps-check` | Normal/build graph exclusion of test-only packages for both Wasm roles and the native bridge |
| `make external-ic-check` | Direct external IC dependency audit; currently refuses existing Platform/Wire migration debt |
| `make tasks`, `make cloc` | Shared maintenance catalog and Rust LOC |

Run relevant focused targets while coding. The maintainer's local overlay requires
`make ci` and Linux `make lint check-secrets` before delivery. The complete gate
runs Clippy before product builds/tests. No standard validation command publishes,
releases or deploys.
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

The [snapshot manifest](../.shared-tooling.snapshot) records Shared Tooling 0.26.18
at `37652c087fff16f55f83d18d27ba17c83f34021c`. The canonical exporter selected a
clean detached copy, consolidated the temporary installer snapshot and retired
unchanged superseded helpers. Imported payloads remain unedited.

The root catalog has `# Production dependencies` and `# Tests and tooling only`
sections, preserved by grouped formatting in development, hooks and release
checks. Normal/build graph exclusions are enforced before managed builds.
Native Agent imports use Platform's optional `agent` feature. Registry CDK calls
and HTTP entrypoints use its `cdk` feature; external HTTP gateway Candid codecs
and method declarations retain the previous representation and decoding policy.
Agent trust, signing, deadlines, saved requests and retry choices remain local.
The registry imports admission/download clients from Blob's client-only package;
the dedicated service component remains owned by Canic's selected adapter.

The common external-IC audit still reports direct Candid uses in the retained
protocol and Canic 0.111 generator integration. CDK/Agent declarations are removed;
labels or a passing test-only exclusion do not establish complete Platform/Wire
migration, a successful build or deployment readiness.

Review the [maintenance catalog](../tasks/README.md) for dependency security,
toolchain/MSRV checks, snapshot drift, README freshness and unused dependencies.
These procedures are available; no recurring schedule is enabled automatically.
