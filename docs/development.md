# Development and qualification

Default dependency declarations select published crates and the root lockfile;
sibling repositories are not build inputs. The latest maintained selections are
Blob Client/Contracts 0.49.0 and Platform 0.6.18. The preserved lock selects
Host FS/Process and Testkit 0.68.7, Memory 0.51.3 and IcyDB 0.284.3. Canic remains
at 0.111.0. These selections were already present before the tooling refresh.

This prepared graph is **not buildable yet**: Canic still owns Memory 0.35, while
new components use Memory 0.51. `make runtime-check` rejects that split. Blob's
service contracts now use IC Wire and no longer implement Candid for the permission
embedded in our public protocol. The focused native check fails there too.
Canic must provide the compatible component/client boundary before qualification;
do not add a DTO mirror, old/new codec fallback or independent bootstrap here.
Blob 0.49 keeps the 0.48 Rust interfaces and stored formats. Its browser assets
now require published `@tokostudios/ic-platform@0.1.0` and the owner's SDK 6.1.1
override; the historical worker recipe is not qualified for these assets. See
[worker qualification](blob-worker.md) before selecting tools. Earlier passing
evidence does not qualify this selected graph.

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
make tools-check
```

Ordered setup prepares the locked project cache before application-tool selection,
including under parallel Make. Cargo/JSON observation failures stop before Testkit
installation. `make deps` remains an explicit cache-preparation command;
`make tools-check` never fetches or installs missing inputs.

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
| `make check`, `make test`, `make clippy`, `make docs-check` | Aggregate native Rust checks across the declared package set |
| `make fmt`, `make fmt-check` | Shared Cargo sorting and Rust formatting |
| `make ci` | Complete native and managed IC delivery gate with retained failure logs |
| `make test-blob-worker` | Opt-in real SDK/Chromium local refusal test with explicit tool configuration ([setup](blob-worker.md)) |
| `make managed-build` | Build all five roles through the matching Canic CLI |
| `make test-ic` | Install registry/blob artifacts and test actual IC admission/restoration |
| `make install-app-tools`, `make app-tools-check` | Prepare/check Canic and Testkit product tools |
| `make test-tool-setup` | Isolated cold-cache, failed-observation and early Clippy-order controls |
| `make msrv-check` | All packages at the required 1.91 floor (source qualification pending) |
| `make package-check`, `make publish-dry-run` | Package and compile both public archives together |
| `make test-publish` | Test publication order, observation guards and interrupted uploads with local substitutes |
| `make install-validation-tools`, `make lint check-secrets` | Linux shell/workflow lint and Git history secret scan |
| `make shared-tooling-check`, `make check-pins` | Snapshot integrity and declared dependency policy |
| `make deployment-deps-check` | Normal/build graph exclusion of test-only packages for both Wasm roles and the native bridge |
| `make external-ic-check` | Direct external IC dependency audit; currently refuses existing Platform/Wire migration debt |
| `make tasks`, `make cloc` | Shared maintenance catalog and Rust LOC |

Select the smallest package or test that answers the current question. For example,
`cargo test --offline --locked -p ic-crates-protocol index::tests` selects the
protocol's index tests, while `cargo clippy --offline --locked -p ic-crates-protocol
--all-targets --no-deps -- -D warnings` checks that package's lint contract.
These checks currently encounter the retained Candid/Wire incompatibility above.
The Make aggregates are broader checks; finish a coherent edit batch before
running the selected checks, and reuse evidence for unchanged inputs under the
[shared validation policy](../DRAGGINZGAME.md#validation-and-evidence).
The maintainer's local overlay requires
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

The [snapshot manifest](../.shared-tooling.snapshot) records Shared Tooling 0.26.20
at `a7f85d45d4212d4c39225f58597580560a333550`. The canonical exporter selected a
clean detached copy and verified all 116 payloads. Imported payloads remain
unedited. The compact baseline links task-specific engineering contracts;
detailed change metrics and symbol inventories are opt-in. Release notes may
retain archived `Draft` headings below the canonical previous release, while
active draft conflicts still refuse.

The npm SDK boundary is independent of Rust Platform adoption. The historical
Blob adapter generator still emits a direct SDK Core import; migrating that
recipe and qualifying current browser/provider assets remain part of
[#12](https://github.com/dragginzgame/ic-crates/issues/12). This snapshot refresh
does not qualify those retained tools or add an npm toolchain to this workspace.

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
