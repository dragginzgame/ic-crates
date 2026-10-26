# Current handoff — 2026-10-26

IC Crates is a controller-operated IC registry and native Cargo bridge. Package
versions remain 0.2.1, with pending 0.3.0 notes; the user owns release operations.
Current Git checkpoint is `2899442`, which committed the tooling/coordination
adoption. The Blob/Platform dependency update remains uncommitted and performs
no real release, publication, deployment or provider operation.

## Shared Tooling adoption

One verified snapshot now selects **0.26.18**, commit
`37652c087fff16f55f83d18d27ba17c83f34021c` (the committed `VERSION` is the version
authority). The canonical exporter imported 114 unchanged payloads from a clean
pinned source and consolidated/retired the temporary selected-installer snapshot.
[AGENTS.md](../../AGENTS.md) identifies the new baseline. No sibling was edited.

- Explicit `IC_TOOLS` setup, offline checking and compact CI evidence select
  `didc ic-wasm icp quill wasm-opt`; the shared catalog selects ICP CLI 1.7.0.
- Common Rust tools are Cargo sort/derive sort. Product selections retain the
  Canic 0.111 declaration extractor 0.1.6 and select IC Build's metadata editor
  0.68.5 with `cli`, debug profile and a 4 MiB document budget. Testkit
  setup/check select `cli`; Testkit owns PocketIC setup and lifecycle.
  [Product pins](../../ci/product-tools.env) are separate from shared catalogs.
- Production and test/tooling dependencies have separate comment-headed groups.
  Grouped formatting is used by hooks, CI and selected-source release checks.
  Normal/build graph checks enforce test-only exclusions for both Wasm roles and
  the native bridge before managed builds.
- IC Build owns manifest/lock rendering; the shared transaction owns Cargo-input
  publication/rollback. The product receipt is validated inside that transaction
  and its optional original is restored on failure. Its existing format is kept.
  Release fixtures qualify receipt-failure rollback, explicit resume, source-bound
  receipts, grouped refusal and committed-source checking against a different live
  manifest. Metadata-editor admission does not read a partially prepared lockfile.
- Development uses focused checks; the maintainer's local overlay retains full
  CI and Linux lint/security before delivery. The complete gate retains every
  check, with Clippy before product builds/tests.
  Normal releases start fresh; recovery remains explicit through `release-resume`.
  [Release details](../publishing.md) describe the user-owned commands.

## Blob and Platform dependency adoption

The registry now selects published Blob Client/Contracts 0.48.0 and imports
admission/download clients from the client-only package. Direct service Core is
removed; Canic 0.111 still owns its older transitive Blob 0.22.16 component.
Platform 0.6.17 supplies native Agent and registry CDK calls/HTTP entrypoints;
all workspace direct Agent/CDK declarations and imports are removed. Features
are selected by the consuming package. Signed-request custody, trust and retries
retain their existing policy. The external HTTP gateway keeps its Candid records,
method declarations and original decoder policy through explicit codec hooks.

## Current blockers and obligations

The maintainer directs shared IC upstream coordination through
[IC Platform #58](https://github.com/dragginzgame/ic-platform/issues/58), which owns
the integration contract and supplying-version matrix. Existing owner issues
retain implementation and consumer acceptance; acknowledgments remain pending.

The selected application dependency cut is **not buildable/release-ready**:
Canic 0.111 selects Memory 0.35.19 while current components select 0.51.3. The
runtime check still refuses. Blob 0.48's permission DTO is IC Wire, while the
retained `PublishRequest` embeds it in Candid and fails compilation. Current
IcyDB/Blob adapter adoption and retirement of old verifier/Candid calls remain
[IC Crates #1](https://github.com/dragginzgame/ic-crates/issues/1), coordinated with
[Canic #444](https://github.com/dragginzgame/canic/issues/444#issuecomment-6410861561),
[#613](https://github.com/dragginzgame/canic/issues/613) and
[#590](https://github.com/dragginzgame/canic/issues/590).

The external-IC declaration audit now refuses only remaining direct Candid
dependencies in the retained protocol and Canic 0.111 generator integration.
These remain migration debt, not exceptions or a claim of policy compliance. The
application-facing owner remains Canic. Test-only graph exclusion passes; that
is not source compilation or artifact/deployment admission.

Keep original archives, permissions, manifests, signed claims, browser profiles,
old executables and provider obligations. Do not reinterpret old Candid jobs with
new codecs, reset uncertain history or redispatch a worker from a Cargo retry.
Historical 0.22-era SDK/browser and earlier compatible managed IC evidence remain
under `target/blob-tooling`, `target/blob-worker-tests` and prior validation runs;
they do not qualify the new graph. Private fixture keys must not be uploaded.

## Evidence and resumption

Current dependency-update evidence is under `target/platform-blob-adoption`.
The standalone API proof reads the actual HTTP records/codecs and uses Platform
0.6.17 with locked Agent 0.49.2: strict Clippy, two native gateway/signed-request
cases and Wasm HTTP entrypoint compilation pass. It does not compile the complete
application or qualify installed Canic composition. Workspace Clippy stops at
the existing `PublishRequest` Candid trait error before application checking.
Focused metadata/pins, deployment exclusions, formatting, links, Linux lint and
secrets pass. Full CI stops at the unchanged Memory split in
`target/validation-runs/validation.fe4ENp`; its complete log is retained alongside
the focused and API-proof logs. Incoming locked Host/Testkit 0.68.6 and Wire
0.7.15 selections are preserved.

Current evidence is retained under `target/shared-tooling-adoption`: exporter
impact/refresh logs, tool installation/admission, deployment exclusions, external
IC refusal, syntax/lint and release/publication fixtures. The fixture deliberately
exercises local Git effects only in disposable repositories. Earlier failed
fixture attempts remain separate from passing attempts.

The final focused Linux run passed all 12 selected targets, including tool/pin
checks, deployment exclusions, formatting, documentation links, lint, secrets and
release/publication fixtures. Its complete logs are in
`target/validation-runs/validation.USBuMM`; the selection and summary are in
`target/shared-tooling-adoption/final-validation.log`. Runtime identity and
external-IC refusals are retained separately; no complete CI pass is claimed.
The required `make ci` attempt stopped at the unchanged Memory identity refusal
after snapshot, tools and metadata checks passed; its logs are in
`target/validation-runs/validation.h38INZ` and
`target/shared-tooling-adoption/full-ci.log`.

The initial dirty state and previous handoff/supplement are preserved in the
recorded adoption attempt. Incoming lock updates to Wire 0.7.15 and Platform
0.6.17 were preserved and fetched locked; this adoption changes catalog grouping,
not application version requirements. Recheck the live lockfile before extending
validation while concurrent dependency work is active.

Focused Linux checks do not qualify native macOS or the full release gate.
The Ubuntu/macOS 15 Intel/Apple Silicon CI matrix remains required. Next work is
current Canic/Platform/Wire application adoption, then matching native/package/
managed IC checks and the real archive/provider/catalog/gateway journey.
