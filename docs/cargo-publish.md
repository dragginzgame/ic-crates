# Cargo upload bridge

The retained implementation below predates Blob 0.47. The current dependency
selection fails compilation at its embedded Candid permission type and needs
coordinated Canic/IC Wire and trusted-registration adoption. Preserve old tools
and jobs; these setup and recovery instructions do not qualify the new graph.
See [current dependency evidence](status/current.md).

`ic-crates` receives Cargo's publish API while large archives bypass IC
message bodies. The registry canister continues to own admission intent and the
final catalog transaction. Blob Storage continues to own provider transfer,
verification, browser journals, references and accounting.

The implementation follows Cargo's [publish API](https://doc.rust-lang.org/cargo/reference/registry-web-api.html)
and [index contract](https://doc.rust-lang.org/cargo/reference/registry-index.html).
Upload framing is little-endian metadata length, JSON, archive length and archive.
Unknown Cargo presentation fields are ignored. Renames map
`explicit_name_in_toml` to index `name` and original `name` to `package`;
`version_req` becomes `req`. Explicit self-registry URLs become null; other
registry URLs remain explicit. Extended feature syntax receives index schema 2.

## Explicit setup

Install/enroll/configure the registry and Blob Service first. Build the native
bridge with `cargo build --locked -p ic-crates`. Create a private
absolute spool directory with mode 0700 and a mode-0600 token file containing
exactly 32 independently random **binary** bytes. Cargo uses the lowercase
64-character hex representation of those bytes, supplied through `cargo login`
or its credential provider. Never commit tokens, keys or upload journals.

Supply one JSON configuration file, replacing every example path and principal:

```json
{
  "listen": "127.0.0.1:8080",
  "public_origin": "https://registry.example",
  "replica_url": "https://icp-api.io",
  "root_key_file": null,
  "identity_file": "/private/operator-ed25519.pem",
  "token_file": "/private/cargo-token.bin",
  "registry": "REGISTRY_PRINCIPAL",
  "blob_service": "BLOB_SERVICE_PRINCIPAL",
  "namespace": 1,
  "uploader": "UPLOADER_PRINCIPAL",
  "spool": "/private/ic-crates-spool",
  "worker": "/private/reviewed-blob-worker",
  "worker_timeout_seconds": 600,
  "permission_seconds": 900,
  "max_archive_bytes": 10485760,
  "max_retained_jobs": 1000,
  "max_spool_bytes": 1073741824
}
```

Run `target/debug/ic-crates /absolute/path/bridge.json`. The PEM must
sign as a registry controller; it does not impersonate the canister tenant or
uploader. Ed25519 PEM is the supported bridge signer format. The worker selects
its separate uploader/verifier identities. Bind only loopback; place a trusted
HTTPS reverse proxy in front, with request-size limits, upload/response deadlines,
bounded concurrency and an overall deadline covering worker plus catalog calls.
Configure request body limits to `max_archive_bytes + 262152` bytes. The bridge
process serves jobs serially. The proxy must not log Authorization headers or
retry upload requests automatically. Read routes are public.

Remote replicas use the Agent's IC root of trust. Loopback replica testing needs
an explicit `root_key_file`; the bridge never fetches and adopts an unknown key.
Only HTTPS or explicit loopback-IP HTTP origins are accepted. Configuration paths
and parent directories remain operator-custodied; no untrusted namespace writers
or symlinks are supported.

Configure Cargo with the bridge origin using the [README example](../README.md#cargo-configuration).
The bridge's `/index/config.json` advertises that same origin's upload API and
download proxy, after checking the configured catalog is serving. Replica reads
use consensus until certified query support exists.

## Selected Blob Storage worker contract

The worker is an explicitly reviewed absolute executable. It receives **one**
argument: the absolute original job directory. It receives no shell command or
Cargo-provided argument string. This repo supplies the HTTP bridge, canonical provider inputs and
`ic-crates-blob-worker`, a thin composition of the maintained Blob owners.
The operator supplies explicit tools, browser installation and signers through
the [worker setup guide](blob-worker.md). There is no pretend provider or default uploader
in the production binary.

Before admission or worker dispatch, durable owner-only files contain:

| File | Meaning |
| --- | --- |
| `archive.crate` | Exact received, structurally validated archive |
| `permission.candid` | Original `UploadAdmissionRequest` with registry tenant, uploader, expiry and IDs |
| `manifest.candid` | Original `UploadManifestRequest`, including upstream-computed leaves and headers |
| `publish.candid` | Exact `PublishRequest` for subsequent catalog confirmation |
| `intent.json` | Configuration, signing principal, root/worker digest and canonical index entry |

The shared Caffeine manifest builder hashes the same 64 KiB frames written to the
archive. Hashed headers are exact `Content-Length` and
`Content-Type: application/gzip`; no filename/cache hints are inferred.
The upstream SDK's rebuild must reproduce those headers, leaves and root.

The driver must prepare `manifest.candid` as the exact uploader, transfer the
frozen archive through the maintained browser SDK, independently observe provider
bytes as the installation's verifier, then submit the saved attestation once.
Use upstream [`prepare-upload`, `observe-upload` and `submit-attestation`](https://github.com/dragginzgame/ic-blob-storage/blob/main/docs/operator-guide.md)
and its [selected-signer/browser launcher](https://github.com/dragginzgame/ic-blob-storage/blob/main/clients/browser/README.md#launch-chromium-from-a-native-parent).
Keep all provider/browser/signed journals beneath the original job or another
independently retained bound directory. The driver owns its child-process cleanup
and bounded diagnostic files; HTTP responses redact its stdout/stderr. It must
never regenerate IDs, grant as an impersonated tenant or reset a browser profile.

A zero exit allows a catalog check, not success. The catalog authenticates the
configured verifier's raw digest and a currently live first reference. It rejects
missing/fenced/conflicting evidence. A nonzero exit or deadline returns a Cargo
503 and preserves the worker claim. Real SDK preparation and Chromium/native failure paths have local evidence;
accepted service/provider/verifier/catalog completion still needs qualification.

## Validation and recovery

Metadata is capped at 256 KiB and canonical index entries at 64 KiB. Archive
validation requires exact framing/EOF, a valid single gzip stream and CRC,
nonduplicate safe paths below `name-version/`, regular files/directories only,
at most 4,096 entries, at most 256 MiB expanded data, and one matching bounded
Cargo manifest. Archive name, version, links and Rust version must agree with the
submitted metadata. Archives are never extracted or executed by this server.

Job identity is the canonical crate namespace plus exact version. Admission and
worker claims survive before dispatch. The IC transport saves the exact signed
update before sending it and retains replies; it disables TCP-error redispatch.
A lost reply, expired ingress, empty claim or worker timeout never authorizes
another upload. Partial jobs refuse; preserve them for inspection.

Re-publishing identical bytes/metadata with the identical selected scope checks
`registry_publish` with the original permission, without repeating admission or
worker execution. The final transaction is exact and idempotent. A changed scope,
worker binary, metadata or archive is a conflict. Each catalog attempt retains
its own signed request and receipt, with a ceiling of 100 attempts per job.
Failed admission/setup/transfer requires explicit operator reconciliation through
Blob Storage's original journals; an absent attestation is not a retry permit.
The spool owns retained-archive and job-count budgets and never evicts old evidence.
Budget additional space for metadata, signed replies, transient incoming archives
and the worker's independently bounded profiles/journals.

`cargo test -p ic-crates` exercises framing, malformed archive rejection,
worker failure and lost-catalog-reply recovery. Its real Cargo fixture publishes,
polls the index, downloads and compiles a consumer against a loopback substitute.
It performs no production catalog mutation, real provider effect or crates.io
publication. Native IC/provider deployment and general publisher accounts are
separate qualification work.
