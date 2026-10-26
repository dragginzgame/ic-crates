# Current handoff — 2026-10-26

IC Crates is a controller-operated IC registry and native Cargo bridge. Package
versions remain 0.2.1, with pending 0.3.0 notes; release operations are user-owned.
Current Git checkpoint is `3447fcd`. The setup, release-fixture and Host repairs,
dependency selections and Shared Tooling adoption are uncommitted. No real release, publication,
deployment or provider operation has been performed.

## Current Blob selection

Root declarations and the released lock select **Blob Client/Contracts 0.49.0**
and **Platform 0.6.18**. The incoming lock already selected Host FS/Process/Testkit
0.68.7, Wire 0.7.16, IcyDB 0.284.3 and Memory 0.51.3; the tooling adoption
preserves those bytes. Both published Blob artifacts record supplying commit
`9777e07b5d2eb281bef0cdd9793dba887f6505f7` in their Cargo provenance.
Registry admission/download imports use the client-only package;
direct Blob Core is removed. Canic 0.111 still owns its older mounted Blob
0.22.16 component. A new client selection does not upgrade that component.

The published 0.49 and 0.48 client/contracts Rust source trees are identical.
Blob's browser assets now consume published `@tokostudios/ic-platform@0.1.0`,
checked identity restoration and the retained Caffeine SDK 6.1.1 override. Our
[historical worker recipe](../blob-worker.md) retains the retired 0.22-era
Candid/verifier flow; it is not qualified with new CLI/browser assets. Keep its
original tools, profiles, permission bytes, manifests and signed uncertainty.
Never redispatch from a Cargo retry or infer completion from a worker exit.

## Current blockers and acceptance

The selected application graph is **not buildable or release-ready**. Canic
0.111 selects Memory 0.35.19 while current components select 0.51.3; runtime
validation refuses. Blob's owned Wire permission has no Candid implementation,
but the retained public `PublishRequest` embeds it in Candid. The current native
protocol check fails there. IcyDB participant wiring and retired verifier calls
also await current adapters. Do not add DTO mirrors, codec fallbacks or another
lifecycle/provider owner.

Authored native Agent and registry CDK APIs/HTTP entrypoints use Platform, with
package-selected features. The external HTTP gateway retains its Candid records,
method declarations and original decoder policy through explicit codec hooks.
Canic 0.111's Blob macro still generates SDK attributes requiring consumer
`::ic_cdk`; Blob canister checking fails E0433 without that removed dependency.
The compatible Platform-based generator/adapter remains Canic-owned.

[IC Crates #12](https://github.com/dragginzgame/ic-crates/issues/12) owns current
client adoption; [#10](https://github.com/dragginzgame/ic-crates/issues/10) owns
mutation/retained-identity migration and
[#1](https://github.com/dragginzgame/ic-crates/issues/1) owns the real
archive/provider/catalog/gateway journey. Related Canic work remains
[#444](https://github.com/dragginzgame/canic/issues/444),
[#613](https://github.com/dragginzgame/canic/issues/613) and
[#590](https://github.com/dragginzgame/canic/issues/590).
[Platform #58](https://github.com/dragginzgame/ic-platform/issues/58) retains
historical coordination evidence; its maintainer closed the central queue and
withdrew acknowledgement gates. Actual implementation and consumer acceptance
remain in the existing owner issues.

## Pending local consumer repairs

Locked cache preparation precedes offline application-tool selection, including
parallel Make. Cargo failures and invalid JSON stop before Testkit installation;
the isolated fixture also preserves early, fail-closed Clippy ordering. Release
fixtures own their patch draft and retain actual missing-note refusal. Private
job writes use Host's create-only 0600 publication, and the same acquired spool
descriptor remains held across the publisher critical section. Owners remain
[#9](https://github.com/dragginzgame/ic-crates/issues/9),
[#4](https://github.com/dragginzgame/ic-crates/issues/4) and
[#6](https://github.com/dragginzgame/ic-crates/issues/6).

Evidence under `target/consumer-repairs` includes actual tool setup, pending-note
and finalized-only disposable release fixtures, and a source-reading Host proof
using released 0.68.6. Strict Clippy and three maintained Host tests pass; this
proof does not compile the protocol-blocked publisher. The focused Linux run
passed 12 targets in `target/validation-runs/validation.qcceWQ`; full CI
`target/validation-runs/validation.Z3X8nh` stopped at the Memory split.

## Tooling and qualification

The verified Shared Tooling snapshot is **0.26.20**, revision
`a7f85d45d4212d4c39225f58597580560a333550`, with 116 unchanged payloads.
The compact baseline links task-specific contracts; detailed change accounting
is opt-in. The explicit local full-CI/Linux lint requirement remains in force.
Explicit tools, grouped Cargo declarations, deployment graph exclusions and
transaction-owned metadata/receipt recovery are described in
[development](../development.md) and [publication](../publishing.md).
Current adoption evidence is under `target/shared-tooling-adoption/0.26.20`.
The consumer release fixture passes archived-note byte preservation, active-draft
refusal and existing recovery checks. Linux lint/secrets pass. Required `make ci`
stops at offline tool admission: incoming Testkit 0.68.7 lacks a prepared server
selection. Locked dependency fetching passed; package versions and lock bytes are
unchanged. The existing Memory/Candid blockers still prevent full qualification.
The historical Blob adapter's direct SDK Core import remains part of the
unqualified browser recipe in [#12](https://github.com/dragginzgame/ic-crates/issues/12).

Earlier API evidence under `target/platform-blob-adoption` reads actual HTTP
records/codecs with Platform 0.6.17: strict Clippy, gateway/signed-request tests
and Wasm HTTP exports pass. This does not qualify complete application composition.
Historical compatible managed IC and SDK/browser evidence remains under
`target/blob-tooling`, `target/blob-worker-tests` and earlier validation runs;
those runs do not qualify the current graph. Private fixture keys must not be
uploaded. Preserve all existing artifacts and failure evidence.

Earlier Blob 0.49 evidence is under `target/blob049-adoption`: published-source
comparison and locked fetch, native Rust 1.91 and Wasm client/contracts checks,
six focused targets and Linux lint/secrets pass. The full CI attempt stops at the
Memory split in `target/validation-runs/validation.pRfmM9`; the protocol check
retains the owned Candid/Wire error. Resume by adopting compatible released
Canic/Platform/Wire adapters, then qualify complete native/MSRV/packages, managed
IC and the real provider journey.
Linux checks do not qualify macOS 15 Intel/Apple Silicon; the complete native
CI matrix and full application gates remain required.
