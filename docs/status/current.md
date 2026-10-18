# Current handoff — 2026-10-13

[IC Crates](https://github.com/dragginzgame/ic-crates) contains a controller-operated
IC registry and native Cargo upload bridge. The user released **0.2.1**, tag
`v0.2.1`, commit `aea8719`. Package versions remain 0.2.1. The user committed worker preparation at `e878337`; the pending
worker batch is recorded under pending **0.3.0** because new archive manifests
change their hashed MIME type. The user owns staging, commits, releases,
publication, deployment and paid effects.

## Implemented behavior

Canic owns lifecycle and memory bootstrap; IcyDB owns the catalog. Blob owns
permissions, provider protocol, browser journals, verification, references and
accounting. Registry endpoints currently require a controller. The native bridge
validates Cargo framing/archive structure, retains exact original jobs before
effects and confirms authenticated verifier checksum plus a live reference before
catalog publication. Cargo retries never dispatch admission or a worker again.
Yanking retains published references and downloads.

The new `ic-crates-blob-worker` composes maintained Blob preparation, browser
transfer and independent verifier commands for an **already-admitted canister
tenant permission**. Explicit tool hashes, signer principals, installation scope
and budgets are retained before effects. The worker refuses a reused evidence
folder; pending/refused/uncertain outcomes stop. Selected worker executables
are bounded at 256 MiB, admitting the supported debug build. A successful worker still requires
catalog confirmation. There is no second tenant grant, provider encoder, SDK
hashing implementation, transfer journal or retry owner.

An offline preflight checks original requests/bytes and real SDK/native manifest
agreement without service/provider calls. New archive manifests use exact
`Content-Length` and `Content-Type: application/gzip`, matching the SDK's detected
archive type. Old octet-stream manifests and jobs must retain their original
worker and journals for explicit reconciliation; do not rewrite admitted claims.
Serialized job/configuration layouts are unchanged, but the new manifest behavior
is a pre-1.0 semantic hard cut.

See [bridge setup and recovery](../cargo-publish.md),
[explicit Blob tool/signers and local test setup](../blob-worker.md),
[development](../development.md) and [user-owned publication](../publishing.md).
Publisher accounts, ownership, search, browsing, certified queries and caching
remain unfinished. The README pins [rust-lang/crates.io](https://github.com/rust-lang/crates.io)
as the upstream port reference; existing adaptations and MIT attribution remain
[packaged with the protocol](../../crates/ic-crates-protocol/UPSTREAM.md).

## Current graph and IC blocker

| Owner | Locked selection |
| --- | --- |
| Canic family | 0.111.0 |
| IcyDB family | 0.273.0, selected by the user |
| IC Blob Storage/contracts | direct 0.23.1; Canic adapter retains 0.22.16 |
| IC Memory | 0.35.19 through Canic; 0.36.2 through the updated components |
| IC Timers | 0.17.14 through Canic; 0.21.4 through IcyDB |
| Direct harness IC Testkit | 0.35.1; Canic/testing retains 0.33.8 |
| Native IC Agent | 0.49.2 |
| Direct native Host FS/Process | 0.17.1 |
| Shared Tooling | 0.9.5, `b1630b89482cc6071fc928a86962d79c9c11fdb7` |

The selected graph deliberately awaits compatible Canic adoption. The unchanged
runtime gate rejects both Memory and Timers splits and now reports their versions. `make ci` passes snapshot, tools and
metadata checks, then fails `runtime-check`; current managed builds/PocketIC
qualification are pending. No downgrade, local sibling override or weakened gate
was introduced. Host tooling's other FS/Process versions are outside the canister
runtime. The Testkit installer selects the harness's resolved direct dependency,
using Shared Tooling's canonical installer and Testkit-owned server setup rather
than assuming every upstream SDK has the same Testkit version.

Current input SHA-256:

- Cargo.lock: `10da2eaa5b0f0f8147cc3d4e0e2269d7f38df0a5fd89943e7b854d0ea2238f26`.
- Cargo.toml: `7c56b89c2e49c72bca5f0c21b3324bdbb8e8fae46c3785fcf296e87de32d2b84`.

Normal-Wasm inverse traces confirm Canic Core 0.111 owns Memory 0.35.19 and
Timers 0.17.14; direct Memory/Blob/IcyDB own Memory 0.36.2 and IcyDB owns
Timers 0.21.4. The current diagnostic is retained at
`target/runtime-current-diagnostic.log`. The fresh full CI attempt at
`target/validation-runs/validation.gQGRP5` passes snapshot/tools/metadata and
refuses both runtime splits. Shell/workflow lint, secret scan, formatting and
document links pass in `target/runtime-diagnostic-static-checks.log`. Registry lookup cannot find Canic 0.112.0;
GitHub's latest tag remains 0.111.0. Framework adoption must precede qualification.
The intended application boundary is Canic; current direct Blob client/CLI paths
remain adapter work, not proof of an entirely Canic-facing integration.

## Earlier local evidence — prior dependency graph

The evidence below used IcyDB 0.272, Blob/contracts 0.22.15 and native Host 0.15.2.
Its Cargo.lock SHA-256 was `4fc1c2171d6a2edf6b309c21a3bef100643c0221314b7c79c71e793a75d5af89`;
Cargo.toml SHA-256 was `c6f91e4a628b1e6e138a357e995f2b4d6b76522572e974a33caa87beac5511a4`.
These checks and the frozen Blob 0.22.14 tool route do not qualify the newly
selected 0.273 / 0.23 / Host 0.17 graph.

All 13 separately requested remaining native delivery targets passed: metadata,
pins, links, formatting, complete native check/tests, strict Clippy, rustdoc,
protocol Rust 1.88/application Rust 1.91, verified public package archives, release
command checks, and release/publication fixtures. Actual unmodified Cargo still
publishes, resolves, downloads and compiles through a loopback registry substitute.
That fixture includes renamed optional dependencies and transitive feature chains.
Linux ShellCheck, actionlint, Git-history Gitleaks, Node syntax and diff checks pass.

Retained evidence:

- Full CI runtime refusal: `target/validation-runs/validation.PmcR7q` and
  `target/blob-worker-ci-refreshed.log`.
- Native targets: `target/blob-worker-native-validation.log`; affected final checks
  also pass at `target/validation-runs/validation.GtiIEC`. Exact input guard
  baseline retained in `target/blob-worker-native-inputs.json`.
- Static checks: `target/blob-worker-static-checks.log`.
- Final real-tool integration: `target/blob-tooling/integration-test-compiled-worker.log`,
  private attempt `target/blob-worker-tests/preflight.gQjEqZ` (97.31 seconds).
  The earlier multi-leaf case is retained separately (85.79 seconds).

The worker integration installs clean released Blob **0.22.14** source at
`54d8186d8db47fbb00ec7531163f34f45ad2f2f9` into an owned prefix using the documented
source-install recipe. Its CLI, paired owner browser bundles, patched Caffeine
SDK 1.1.2, ICP SDK 5.4.0, Playwright 1.63.0, Chromium and Node 24.21.0 are selected
explicitly. The adapter selection/hashes remain in
`target/blob-tooling/adapter-0.22.14-final/selection.json`; installation/build logs
remain under `target/blob-tooling`. Dirty sibling work was not adopted.

The real SDK reproduces the native declaration across multiple 1 MiB leaves for
an incompressible archive. A real 135 MiB compiled executable exercises the
worker identity bound. Preflight makes zero loopback requests; modified tools
or archive bytes refuse. Real Chromium opens the original bound private profile,
then an unauthenticated loopback IC substitute refuses certificate dispatch.
The real native CLI retains its exact signed request, reports an uncertain outcome
with zero provider requests, and makes exactly one service request. The worker
stops before browser transfer/verifier/attestation and refuses redispatch without
another request. The browser profile and private original native evidence survive.
Fixture signer secrets remain private under the attempt directory; do not publish
or upload it as a public CI artifact.

The initial octet-stream SDK mismatch and sandbox loopback refusal remain in
separate earlier logs. They are corrected/rechecked attempts, not passing evidence.
No accepted preparation/provider transfer/attestation/catalog journey is claimed.
Linux execution does not qualify macOS. Process-group cleanup does not guarantee
containment of detached descendants after forced parent termination; original
profiles and journals remain mandatory for inspection/reconciliation.

## Historical IC evidence and next acceptance

On the earlier compatible Canic 0.111 / IcyDB 0.270.3 graph, all 18 delivery targets
passed at `target/validation-runs/validation.6DKFXF`. Matching all-role Canic build
identity was `4c742c6a12fbb6a8510b3b30ad9499b3d90031da829f2f49b0edc043988d6f73`.
Actual Registry/Blob Wasm tests covered target initialization, controller rejection,
immutable configuration, registry-originated tenant admission, exact replay,
conflicting intent/unattested publication refusal and same-release restoration.
These results do not qualify the current IcyDB 0.273 graph.

[Canic #33](https://github.com/dragginzgame/canic/issues/33#issuecomment-6145063926)
owns coordinated Timers adoption. [IC Crates #1](https://github.com/dragginzgame/ic-crates/issues/1)
owns consumer completion: qualify matching managed artifacts after adoption, then
one exact archive through acknowledged preparation, real provider completion,
configured verifier, live reference and registry publication; consume it through
the HTTPS gateway with unmodified Cargo. Interruptions/lost replies must reconcile
original identities without another uncertain transfer. The existing
[Blob #4](https://github.com/dragginzgame/ic-blob-storage/issues/4#issuecomment-6143637031) tracks maintained
prior-admission tooling and consumer acceptance. Production Fleet deployment,
cross-release retained obligations and paid retention remain separate.

No real checkout index/ref, package version, release, publication, deployment,
paid/live provider effect or sibling source was changed in this batch.


## Prepared Blob client boundary — 2026-10-18

The isolated contribution replaces the registry's two Core client imports with
`ic_blob_storage_client::{admission, download}` and selects Client/Contracts 0.35.
Blob owns these unchanged replicated calls and codecs; registry intent, catalog
policy and Canic endpoint/lifecycle ownership remain local. The native bridge
and worker selection are unchanged. Core still exists transitively through the
actual Canic Blob component; this does not remove its service implementation.

Client 0.35 is not yet published. The committed lockfile is deliberately retained
until the family is available, so this draft is not a locked runnable adoption.
Before merging, refresh the real registry lock against the published family and
qualify the coherent Canic/IcyDB graph. No path overrides belong in that lock.
The existing runtime handoff remains with PR #7; this contribution does not
supersede its framework or storage work.

A captured candidate using the exact proposed Client/Contracts sources compiles
those packages and then reaches three Memory type mismatches in the registry.
The unchanged registry source against its original released dependencies reports
the same three mismatches: this adoption does not establish a new regression or
a passing managed build. Candidate source patches qualify only the call shape,
not publication or deployment. Focused logs and failed prerequisites remain in
the Blob owner's `.tmp/review-validation/client-boundary0350/`. No full CI,
installed registry journey, native macOS, publication or deployment is claimed.
