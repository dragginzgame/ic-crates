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

## Validation cadence during shared-policy adoption

Shared Tooling owns the [validation policy](DRAGGINZGAME.md#validation-and-evidence);
local guidance supplies commands and product qualification requirements only.
Maintainer-approved exception (2026-10-14): until this snapshot adopts Shared
Tooling's focused-handoff policy, replace its full-suite-before-handoff requirement
with focused checks of changed behavior and directly affected callers, including
relevant failure/recovery paths. Reuse unchanged passing evidence. Documentation
changes need link, consistency and diff checks only. Run local full suites only
for an explicitly authorized release or an explicit request for those checks;
implementation, continuation, PR preparation and readiness requests do not qualify.
Report tested scope; an unrun full suite alone is not a handoff blocker. Configured
CI, required branch checks, release gates and separate effect authority remain
unchanged. This exception avoids redundant local testing; it does not claim a
snapshot refresh. On reviewed adoption, remove this exception and retain only the
link to the canonical policy and repository-specific commands.
