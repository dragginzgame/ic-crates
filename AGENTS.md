# IC Crates contributor instructions

Read [the current handoff](docs/status/current.md), then apply the reviewed
[Dragginzgame baseline](DRAGGINZGAME.md). Shared Tooling 0.9.5 revision
`b1630b89482cc6071fc928a86962d79c9c11fdb7` is recorded in
[.shared-tooling.snapshot](.shared-tooling.snapshot). This file is the local overlay.

- Work in this repository; sibling checkouts are read-only unless separately authorized.
- Canic owns lifecycle, memory bootstrap and Fleet deployment. IcyDB participates
  synchronously. Blob Storage owns upload/provider state, verification, references
  and accounting. Do not duplicate those owners here.
- Default builds use released dependencies and the root lockfile. Each canister
  graph needs one Memory and one Timers identity. Explicit local development
  overrides cannot qualify publication or releases.
- `ic-crates-protocol` and the native `ic-crates` CLI/library are crates.io
  publication candidates; canister packages remain private. Release preparation
  does not change versions or Git refs. Publish the protocol dependency first.
- Use edition 2024 and directory modules. Keep DTOs passive, records in `design/`,
  catalog operations in `ops/`, orchestration in `workflow/`, and auth at endpoints.
- Persist exact upload intent before dispatch. A missing reply never proves safe
  retry. Yanking never releases published blob references.
- The native bridge uses IC Host FS for durable files/locks and Blob Storage's
  manifest builder. Its selected provider worker owns SDK/browser/verifier journals.
  Never replay that worker from a Cargo retry or infer completion from its exit code.
- Full release checks are `make ci` and Linux `make lint check-secrets`;
  focused development/handoff follows the validation cadence below.
  Support Linux x86-64 and macOS 15 on Intel and
  Apple Silicon; Linux evidence does not qualify macOS.
- Check active builds before editing/building. Preserve source, tools, artifacts,
  failure evidence and unrelated changes. Readiness does not authorize release targets.
- GitHub issues track follow-up work; status documents summarize evidence.

## Validation policy

Apply Shared Tooling's [canonical validation cadence](https://github.com/dragginzgame/shared-tooling/blob/4256171b5b7ec4c4cc4078548c1c1617ee150699/DRAGGINZGAME.md#validation-and-evidence),
adopted by exact revision `4256171b5b7ec4c4cc4078548c1c1617ee150699` (0.12.5).
This revision-bound policy reference supersedes only the validation cadence in
our older snapshot; executable tooling and all other shared rules retain their
recorded snapshot revision. Local guidance supplies commands and product
qualification requirements. Configured CI and release gates remain unchanged.
On snapshot adoption of this policy, replace this separate reference with the
local baseline link; do not keep a duplicate cadence or temporary exception.
