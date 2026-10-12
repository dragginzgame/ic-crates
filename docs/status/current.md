# Current handoff — 2026-10-12

The repository contains a controller-operated IC registry and native Cargo upload
bridge. The public repository is
[dragginzgame/ic-crates](https://github.com/dragginzgame/ic-crates).
The user completed the 0.2.0 Git release (`8b419e5`, tag `v0.2.0`), including the
public `ic-crates` CLI/library packaging. Package publication is a separate
observation. Manifest versions remain 0.2.0; compatible preparation is recorded
under pending 0.2.1. Current changes are uncommitted. The user retains release,
publication and deployment authority.

## Implemented

- Canic lifecycle/memory ownership with synchronous IcyDB participation.
- Journaled immutable configuration, exact upload intents and version catalog.
- Controller-only configuration, admission, inspection, publication and yanking.
- Dedicated target-initialized Canic Blob Service shell.
- Bounded authenticated attestation, raw SHA-256 check and live-reference observation before atomic publication.
- Cargo sparse index, blob redirects and HTTP query-to-update upgrade.
- Token-authenticated native Cargo bridge with bounded framing, archive validation and metadata translation.
- Upstream manifest hashing and IC Host FS durable files/descriptor locks.
- Original archive, permission, scope and signed requests retained before effects; Cargo retries never redispatch the worker.
- Two independently verified public package archives and dependency-ordered user-operated publication adapters.
- Matching Canic managed artifacts and Testkit-owned PocketIC tooling in the standard delivery gate.
- Adapted crates.io rules for underscore-prefixed dependency aliases and transitive extended-feature splitting, with packaged attribution and license notices.

See [bridge setup and recovery](../cargo-publish.md),
[development and managed qualification](../development.md) and
[user-operated release commands](../publishing.md).
The bridge still requires an operator-selected upstream provider worker for the
installation/browser/signers. Publisher accounts, ownership, search, certified
queries and caching remain unfinished. Yanking retains published references.

## Selected released graph

| Dependency | Selection |
| --- | --- |
| Canic, Host and Blob adapter | 0.111.0 |
| IcyDB family | 0.270.3 |
| IC Blob Storage/contracts | 0.22.13 |
| IC Memory | 0.35.18 |
| IC Timers | 0.17.14 |
| IC Agent | 0.49.2, native bridge only |
| IC Host FS | 0.15.0, direct native dependency |
| IC Testkit | 0.33.8, private test harness/tooling only |
| Shared Tooling | 0.9.5, b1630b89482cc6071fc928a86962d79c9c11fdb7 |

Canic 0.111 and IcyDB 0.270 share Timers 0.17; IcyDB 0.271's Timers 0.19
needs coordinated Canic adoption. Every canister graph retains one Memory/Timers
identity. Private host tooling has additional upstream Host FS versions; it is
outside the canister graph. PocketIC's exact dependency requires thiserror 2.0.18;
other pre-existing external package identities were preserved. Siblings remain
read-only and their concurrent changes are excluded from qualification.

## Validation evidence

Native checks cover unmodified Cargo publication of two crates, sparse-index
polling, downloads and consumer compilation against a local substitute. The
consumer resolves an underscore-prefixed renamed optional dependency and enables
it through default, transitive, namespaced and weak features. Recovery covers
failed workers, lost committed replies, conflicting metadata and incomplete
retained intent. Archive tests reject invalid framing, paths, manifest identity,
CRC and bounds. Publication adapter fixtures qualify dependency order, independent
version/checksum/yank observations, interruption and source mutation without
real publication traffic.

The first crates.io adaptations are pinned to
`8ac99b2b31af2efd2ab53035c8636f3c1895654c`; the protocol package includes
[the source attribution](../../crates/ic-crates-protocol/UPSTREAM.md) and upstream
MIT notice. New alias and feature-chain regressions fail on the prior code and
pass after the fix. Reverse traversal preserves cycles and deep feature chains
without a fixed iteration cutoff. Public DTO and storage layouts are unchanged;
existing catalog records and original retained upload claims are preserved.
Retrying a job whose translated feature partition changes still refuses a
different immutable intent; preserve its original bridge executable and journals
for reconciliation rather than rewriting claims or redispatching the worker.

The initial matching Canic build passed for Root, Coordinator, Store, Registry
and Blob. It exposed and fixed the consumer's omitted explicit Canic feature
arrays. Managed installation then exposed IcyDB's separate asynchronous startup
condition: Canic activation does not imply database readiness. Tests allow only
the selected owner's typed pending diagnostic to trigger bounded progression.
The published Testkit helper owns polling; no server pin, lifecycle driver or
provider retry owner was added here.

The actual registry/blob PocketIC test passes: target-bound initialization,
controller rejection, immutable configuration, registry-tenant inter-canister
upload admission, exact replay, conflicting intent rejection, refusal of
unattested publication and same-release restoration of both canisters. It uses
Canic's published synthetic-Root fixture and validates the finalized build
manifest, artifact union, Wasm hashes and Candid endpoints. All 18 current delivery
targets pass, including both public archives, advertised Rust minimums, strict
Clippy, rustdoc and release/publication fixtures. Complete evidence is retained at
`target/validation-runs/validation.6DKFXF`; its managed build/test targets passed
in 44s/20s, with 7.66s of IC test execution. Build attempts are under
`target/managed-builds/attempt.8Dca7F`;
the finalized build identity is
`4c742c6a12fbb6a8510b3b30ad9499b3d90031da829f2f49b0edc043988d6f73`.
Server logs remain under `target/managed-tests/attempt.mdTl7W`. Linux ShellCheck, actionlint,
Git-history Gitleaks and diff checks pass. CI retains failure logs and archives
the complete managed release-build directory through the shared evidence helper.
The verified protocol archive includes the adapted module, source attribution
and exact upstream MIT notice. Earlier managed attempts and the pre-fix Cargo
regression failures remain retained separately.

Consumer evidence was added to existing upstream requests for
[typed IcyDB startup qualification](https://github.com/dragginzgame/icydb/issues/315#issuecomment-6130172650),
[a single Canic pool provider](https://github.com/dragginzgame/canic/issues/514#issuecomment-6130172579)
and [released Blob canister-tenant tooling](https://github.com/dragginzgame/ic-blob-storage/issues/4#issuecomment-6130173055).
Upstream feedback is separate from adopting or qualifying an upstream change.

Linux evidence does not qualify macOS. The workflow configures Ubuntu 24.04 and
macOS 15 on Intel and Apple Silicon. The IC fixture does not prove production
Fleet Root provisioning, cross-release schema/identity upgrades, deployed HTTP
gateway behavior, real provider transfer, browser/verifier acceptance or paid
retention. Standard validation does not publish, release or deploy.

## Next qualification

1. Bind the maintained upstream Blob prepare/browser/verifier worker for a
   canister tenant, preserving the exact already-granted permission and journals;
   [IC Crates #1](https://github.com/dragginzgame/ic-crates/issues/1) owns consumer acceptance.
2. Prove one exact archive through provider completion and registry publication
   in a configured environment, then qualify a gateway Cargo consumer.
3. Qualify production Fleet deployment and cross-release retained obligations
   through upstream lifecycle contracts.
4. Add publisher accounts, ownership, search and browsing.

Preserve original permissions, archives, signed claims, browser profiles and
provider/verifier journals across interruptions. An absent receipt never permits
another transfer. No staging, commit, tag, push, release, provider effect or
sibling edit was performed in this batch.
