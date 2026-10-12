# Current handoff — 2026-10-12

The empty repository now has a controller-operated registry foundation. This
batch adds executable code, focused tests, a locked local dependency graph,
managed app configuration and development instructions. No deployment, provider
effect, release or sibling source edit was performed. The user subsequently
authorized publishing this initial source as the public GitHub repository
`dragginzgame/ic-crates`; this does not change the validation limits below.

## Implemented

- Canic registry component with IcyDB as its synchronous lifecycle participant.
- One shared registry memory allocation pool and one Memory/Timers package identity.
- Dedicated `canic-blob-service` component with target-bound application initialization.
- Journaled configuration, exact upload permission intents and immutable version records.
- Controller-only configuration, upload admission, admission inspection, publication and yanking.
- Bounded upstream attestation decoding with explicit service/namespace/verifier binding.
- Raw archive SHA-256 comparison and a current live-reference descriptor before atomic publication.
- Cargo sparse-index paths, configuration and blob download redirects.
- HTTP query-to-update upgrade until certified query responses are implemented.

The archive is never stored in the catalog. The uploader and verifier still run
through Blob Storage's existing tools; the registry grants permission from its
tenant identity. Published references have no registry deletion/release path.

## Selected source graph

| Dependency | Selection |
| --- | --- |
| Canic and its Blob adapter | Local sibling, 0.111.0 |
| IcyDB family | Local sibling, 0.271.0 |
| IC Blob Storage/contracts | Local sibling, 0.22.13 |
| IC Memory | Local sibling, 0.35.17 |
| IC Timers | Published 0.19.3 |
| IC Metrics | Published 0.7.3 |

The local Timers and Metrics checkouts contain API changes not yet consumed by
Canic. Their published selections avoid `register_once` signature and missing
`record_sample` failures. The initial normal native check stopped on five denied
dead-code findings in Canic's unfinished Auth integration.

## Validation

- Formatting check passes for all three application-owned packages.
- Three protocol tests pass without a lint override.
- All five protocol/catalog/workflow tests passed in a diagnostic build
  with `RUSTFLAGS='--cap-lints=warn'`.
- Both canister components compiled natively in that diagnostic graph.
- Targeted diagnostic Clippy completes with no application-owned warnings;
  the five upstream Canic warnings remain. No lint relaxation is committed.
- Native database tests cover durable configuration, upload intent binding,
  exact publication replay, immutable-version conflicts, namespace collisions,
  HTTP index/download routing and yanking without archive removal.
- Publication evidence tests distinguish provider root from attested raw checksum
  and reject missing/fenced evidence.

The final change gives `registry_publish` an explicit 72 KiB Canic payload budget,
above its default 16 KiB, and bounds canonical serialized index lines to 64 KiB.
Protocol tests and formatting pass after that change. A backend rerun encountered
a temporary syntax error during concurrent Canic edits. Once that syntax was
corrected upstream, the final normal `make check` still stopped on the five Auth
dead-code findings. All five diagnostic tests were then re-run and passed with
the final payload declaration and size bound. The warning cap remains an explicit
diagnostic-only command environment; no committed lint policy is weakened.

These are native composition and state tests. They do not establish live IC
lifecycle, controller rejection, gateway verification, real Cargo consumption,
or real provider completion. No PocketIC or deployment ran. The discovered
Canic CLI is 0.110.54; the app requires a matching 0.111 toolchain for managed builds.

## Next coherent batch

1. Qualify managed Wasm installation and the registry/blob tenant composition
   with matching Canic tooling and IC Testkit-owned PocketIC infrastructure.
2. Add a host-side Cargo publish bridge: bounded archive validation, publish
   metadata translation, direct provider upload and verifier completion.
3. Prove an unmodified Cargo client can resolve and build a crate through the
   gateway; implement certified query index responses and caching.
4. Add publisher accounts/tokens and crate ownership, then search and browsing.

Retain the original upload permissions and package bytes across interrupted
work. An absent receipt is not evidence that a provider upload can be repeated.
Cross-release migration or retained-identity deployment is not promised by this
initial schema; follow each sibling's maintained lifecycle contracts.
