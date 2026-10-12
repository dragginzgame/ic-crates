# Current handoff — 2026-10-12

The repository contains a controller-operated IC registry foundation and a native
Cargo upload bridge. Release preparation now uses a reviewed Shared Tooling
snapshot and released dependencies. The public repository is
[dragginzgame/ic-crates](https://github.com/dragginzgame/ic-crates).
The user completed the 0.1.1 Git release (`8c52898`, tag `v0.1.1`) and reported
protocol package publication. The current packaging changes remain in the working
tree for review; manifest versions stay at 0.1.1. This batch performs no release,
commit, tag, push, package publication, deployment, provider effect or sibling edit.
The user retains all release authority. The pending notes select 0.2.0 because
the native executable/package rename changes build and launch commands.

## Implemented

- Canic registry lifecycle with IcyDB as synchronous participant and one Memory/Timers identity.
- Dedicated target-initialized Canic Blob Service shell.
- Journaled immutable configuration, exact upload intents and version catalog.
- Controller-only configuration, admission, inspection, publication and yanking.
- Bounded authenticated attestation decoding, raw SHA-256 check and live-reference observation before atomic publication.
- Cargo sparse index, blob download redirects and HTTP query-to-update upgrade.
- Native token-authenticated Cargo upload endpoint with explicit IC transport and operator-owned worker boundary.
- Bounded framing, gzip/tar/path checks and matching archive manifest; publish-to-index metadata translation.
- Upstream Caffeine manifest hashing; IC Host FS durable files and exclusive descriptor locks.
- Original archives, permissions, scope and signed requests retained before dispatch; retries never redispatch the worker.
- Coordinated protocol/CLI packaging, pinned dependency/tool policy, host/MSRV CI and user-operated release/publication adapters.
- Publishable `ic-crates` CLI/library with package-local README/license; canisters remain private.
- Dependency-ordered package publication with independent version/checksum observations and retained upload results.

See [bridge setup, worker contract and recovery](../cargo-publish.md),
[development](../development.md) and [user-operated release commands](../publishing.md).
The bridge requires a reviewed provider driver for the operator's browser/signers
and installation; it does not ship a fake provider in the production binary.
`ic-crates-protocol` and `ic-crates` are crates.io candidates. The native binary
is now `ic-crates`, and its library entry point is `ic_crates::run`; the former
development package/binary `ic-crates-publisher` is replaced. Canisters remain private.

## Selected released graph

| Dependency | Selection |
| --- | --- |
| Canic and Blob adapter | 0.111.0 |
| IcyDB family | 0.270.3 |
| IC Blob Storage/contracts | 0.22.13 |
| IC Memory | 0.35.18 |
| IC Timers | 0.17.14 |
| IC Agent | 0.49.2, native bridge only |
| IC Host FS/artifacts | 0.15.0, native bridge only |
| Shared Tooling | 0.9.5, b1630b89482cc6071fc928a86962d79c9c11fdb7 |

Released Canic and IcyDB 0.270 share Timers 0.17. IcyDB 0.271's Timers 0.19
requires coordinated Canic adoption. Fresh-checkout builds need no sibling paths
or warning caps. The sibling Shared Tooling checkout's dirty work was preserved;
89 selected files come from an independently checked clean committed snapshot.

## Validation evidence

Focused native protocol/catalog/bridge tests pass, including an unmodified Cargo
client that publishes to a loopback substitute, polls the sparse index, downloads
and compiles/runs a consuming crate. Recovery checks cover a failed worker, a lost
committed reply, conflicting metadata and incomplete retained intent. Archive
checks reject truncation, trailing bytes, wrong manifest identity, traversal,
symlinks, corrupt CRC and exceeded bounds. Strict targeted Clippy passes.

The prior foundation passed all 15 delivery targets: snapshot/tool integrity, metadata,
runtime identities, pins, documentation links, formatting, native checks/tests,
strict Clippy, rustdoc, both Rust minimums, protocol archive verification and
isolated release-command/recovery fixtures. Ten unit/wire tests and one doctest
pass. Loopback networking is required by the real Cargo fixture; a restricted
sandbox run refused socket creation, and the complete run passed with local
networking enabled. Failure logs remain under `target/validation-failures`;
the standard logger now retains complete runs under `target/validation-runs`.
The new public packages verify together with normal `cargo package --locked`:
the CLI builds against the exact protocol archive in Cargo's temporary staging
registry. Both include their own README and MIT license. Explicit offline batch
verification refused the temporary-registry unpack step; normal Cargo handling
passed without version edits or `--no-verify`. Publication adapter fixtures cover
dependency order, completed-package replay, unknown/mismatched/yanked observations,
lost replies, failed uploads and source mutation, with no real registry traffic.
The final `make ci` passes all 16 targets, including both package archives,
the 1.88 protocol/1.91 application minimums, strict Clippy, rustdoc and
release/publication fixtures. Complete logs and timings are retained at
`target/validation-runs/validation.Y1ZsiX`. Linux ShellCheck, actionlint,
Git-history Gitleaks and `git diff --check` also pass. Manifest versions and
the real checkout's index/commit/tag remain unchanged by this batch.
The reviewed CI matrix configures Ubuntu 24.04 and macOS 15 on Intel and Apple Silicon;
local Linux evidence does not establish macOS qualification. Standard validation
performs no production release or publication. Release adapter tests use isolated
throwaway source/bare-remote Git fixtures, never the real checkout's index or refs.

These are native state/wire/recovery checks. They do not establish installed IC
lifecycle, deployed controller rejection, gateway behavior, real provider transfer,
browser acceptance or operational retention. No PocketIC/deployment ran. The
production bridge signs explicit IC requests, but IC/provider end-to-end acceptance
needs a separately configured environment and reviewed worker. Certified query
index responses, caching and publisher ownership/accounts remain open.

## Next qualification

1. Bind the operator's upstream Blob Storage prepare/browser/verifier driver and
   prove one exact archive journey in a configured IC test environment.
2. Qualify managed Wasm installation and registry/blob tenant composition with
   matching Canic tooling and IC Testkit-owned PocketIC infrastructure.
3. Qualify a real gateway Cargo consumer, then add certified index queries/caching.
4. Add publisher accounts, crate ownership, search and browsing.

Preserve original upload permissions, package bytes, signed claims, browser
profiles and provider/verifier journals across interruptions. An absent receipt
never authorizes another transfer. Yanking preserves published references.
Cross-release retained-identity upgrades are not qualified for the initial schema;
follow upstream lifecycle and obligation-disposition contracts before reinstalling.
