# Blob worker setup and local qualification

`ic-crates-blob-worker` composes Blob Storage's existing native preparation,
browser transfer and independent verifier commands for a registry canister tenant.
It consumes an already-admitted original [Cargo job](cargo-publish.md). It never
grants as the tenant, creates new IDs or retries uncertain effects. The bridge
still confirms publication through `registry_publish` after worker completion.

## Select maintained tools

Use one clean released Blob source selection for the CLI and browser assets.
The exercised selection is **0.22.14**, commit
`54d8186d8db47fbb00ec7531163f34f45ad2f2f9`. Follow its
[supported source installation recipe](https://github.com/dragginzgame/ic-blob-storage/blob/54d8186d8db47fbb00ec7531163f34f45ad2f2f9/docs/local-tools.md#install-the-native-and-browser-tools)
in a consumer-owned checkout/prefix, preserving its Cargo/npm lockfiles and
source identity. Sibling working-tree changes are not tool provenance.

That recipe installs `blob-storage` with locked Cargo dependencies, uses Node 24
and `npm ci --ignore-scripts`, then runs the owner's `tests/browser/build.mjs`.
The owner verifies peer versions and original SDK hashes and applies its maintained
patch to a build copy. Provision the selected Playwright Chromium through the
owner's browser setup. Keep the complete installed Chromium and Playwright trees.

After the owner build, run the consumer's thin Node packaging helper:

```sh
node scripts/dev/bundle-blob-adapter.mjs \
  /absolute/released-blob-source /absolute/new-adapter-directory \
  54d8186d8db47fbb00ec7531163f34f45ad2f2f9
cargo build --locked -p ic-crates --bins
```

The helper bundles the owner's SDK/configuration and callable browser launcher
into `sdk.mjs` and `launcher.mjs`; it records hashes and source selection in
`selection.json`. It adds no provider protocol, hashing implementation, journal
or retry dispatcher. Supplying a commit argument records the operator's selected
identity; it does not authenticate an arbitrary input directory.

## Configure signers, tools and budgets

Create an absolute, private JSON file. Every selected tool is an object containing
`path` (absolute regular file) and `sha256` (lowercase raw file SHA-256).
Compute hashes from the actual installed files, for example with `shasum -a 256`.
Replace every placeholder; this example cannot run as written.

```json
{
  "blob_cli": {"path": "/selected/bin/blob-storage", "sha256": "FILE_SHA256"},
  "blob_version": "0.22.14",
  "node": {"path": "/selected/node", "sha256": "FILE_SHA256"},
  "sdk_module": {"path": "/selected/adapter/sdk.mjs", "sha256": "FILE_SHA256"},
  "launcher_module": {"path": "/selected/adapter/launcher.mjs", "sha256": "FILE_SHA256"},
  "playwright_module": {"path": "/selected/source/tests/browser/node_modules/playwright/index.mjs", "sha256": "FILE_SHA256"},
  "chromium": {"path": "/selected/chromium/chrome", "sha256": "FILE_SHA256"},
  "host_bundle": {"path": "/selected/source/.tmp/browser/publication-host.js", "sha256": "FILE_SHA256"},
  "worker_bundle": {"path": "/selected/source/.tmp/browser/publication-worker.js", "sha256": "FILE_SHA256"},
  "uploader_identity": "/private/uploader.pem",
  "verifier_identity": "/private/verifier.pem",
  "browser_identity": "/private/uploader.json",
  "verifier": "CONFIGURED_VERIFIER_PRINCIPAL",
  "project": "CONFIGURED_PROVIDER_PROJECT",
  "bucket": "CONFIGURED_PROVIDER_BUCKET",
  "provider_origin": "https://CONFIGURED_PROVIDER_ORIGIN",
  "asset_port": 31123,
  "max_body_bytes": 10485760,
  "max_requests": 64,
  "max_request_bytes": 2097152,
  "max_total_request_bytes": 20971520,
  "browser_seconds": 100
}
```

The native signers use Ed25519 PEM. The browser signer uses the selected ICP SDK's
`Ed25519KeyIdentity.toJSON()` format for the **same uploader secret key**. Both
advertised and derived browser principals must match the original permission.
The verifier must match the immutable registry/Blob installation selection.
Project, bucket and HTTPS provider origin must match the service installation.
No ambient login or tenant PEM is used. Preserve private mode-0600 signers and
configuration and mode-0700 parent directories with no untrusted writers/symlinks.

The worker checks selected hashes and signer principals before each phase.
Playwright's transitive modules and Chromium resources remain operator-custodied;
the entry-file hash does not seal their entire installation. Preserve the complete
selected trees and npm lockfile. Use a real absolute Node executable, resolving
any installation symlink before selecting it.

`asset_port` is a fixed unused loopback port: changing the browser origin changes
IndexedDB history. The browser budget is 1–240 seconds. The original bridge
deadline must be at least `browser_seconds + 240`, and at most 600 seconds; use
600 with a 900-second permission for the example above. Hashing/tool startup also
consumes the deadline. Provider request and byte budgets are checked by the owner;
choose them for the selected archive size and provider protocol.

Set the original bridge configuration's `worker` to the absolute
`ic-crates-blob-worker` executable (at most 256 MiB, including debug builds). Set this environment variable **on the bridge
process** so its selected child inherits it:

```sh
export IC_CRATES_BLOB_WORKER_CONFIG=/private/blob-worker.json
ic-crates /private/bridge.json
```

## Offline preflight and retained outcomes

Inspect a fully retained original job without IC/provider requests:

```sh
ic-crates-blob-worker preflight /private/blob-worker.json \
  /private/spool/example@0.1.0 /private/new-preflight-output
```

This verifies original canonical requests, scope, archive checksum, worker/trust
identities and the **real SDK** manifest against the native declaration. SDK
preparation disables fetch. It creates private evidence in a new output directory;
it does not change the original job or create a browser profile. New archives use
exact `Content-Length` and `Content-Type: application/gzip`, with no filename or
cache hint. The SDK also detects gzip when given an octet-stream hint, so the old
octet-stream manifest cannot be replayed under this worker. Preserve old claims
and their original executable for explicit reconciliation; never rewrite them.

Normal dispatch creates `JOB/blob-worker` once. It retains selected inputs and
the SDK manifest before any effects, then:

1. Runs `prepare-upload` as the original uploader and requires an authenticated,
   acknowledged response bound to the original manifest.
2. Calls the maintained Chromium launcher once with a new bound private profile
   and the original permission. Blob's IndexedDB journal owns transfer uncertainty.
3. Runs `observe-upload` as the configured verifier and checks its saved statement
   against the exact permission and raw Cargo checksum.
4. Runs `submit-attestation` once using that original observation directory.

Each phase has bounded output and a process-group deadline. Normal browser cleanup
closes its context. Process-group cleanup does not guarantee containment of a
detached descendant after forced parent termination; inspect remaining processes
and retained history before reconciliation. Tool output stays in private evidence
and is redacted from Cargo responses. A pending/refused/uncertain phase stops.
An existing worker directory refuses redispatch, including after partial failure.
The worker offers no automatic resume/retry command: use upstream inspection and
reconciliation against original native/browser records. Never reset a profile to
make an uncertain job appear fresh. `completed.json` acknowledges attestation,
**not catalog publication**; the bridge's authenticated catalog check owns success.

## Local evidence and remaining acceptance

Select the same tool JSON for the opt-in integration test; signer paths are replaced
with ephemeral fixture keys and the IC URL with a refusing loopback substitute:

```sh
export IC_CRATES_BLOB_TEST_CONFIGURATION=/absolute/selected-tool-template.json
make test-blob-worker
```

The test uses the real released CLI, patched SDK, Playwright and Chromium. It
checks manifest equality across multiple provider leaves, zero-request preflight,
input/tool mutation refusal, retained browser history, one uncertain signed
native dispatch, and refusal of redispatch. Attempts remain private under
`target/blob-worker-tests`, including ephemeral fixture secrets. Do not upload
that directory as a public CI artifact. The normal native Cargo fixture separately
publishes and consumes with unmodified Cargo against a registry substitute.

This is Linux local execution, not deployed service/provider completion or macOS
qualification. The selected IcyDB 0.284 / Blob 0.47 / Canic 0.111 graph has
incompatible Memory identities and a Candid/IC Wire integration gap. The runtime
gate and source qualification remain closed. The frozen Blob 0.22.14 tool setup
above is historical evidence; it is not qualified for the new contracts. Preserve
those tools and original job evidence for reconciliation. Actual prepared
certificates, provider transfer, accepted attestation, live-reference/catalog
commit, gateway consumption and interruption reconciliation remain
[IC Crates #1](https://github.com/dragginzgame/ic-crates/issues/1) acceptance work.
