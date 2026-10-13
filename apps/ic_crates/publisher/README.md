# IC Crates

An operator-run Cargo upload bridge for the
[IC Crates registry](https://github.com/dragginzgame/ic-crates) on the Internet
Computer. This package provides the `ic-crates` executable and the Rust library
entry point `ic_crates::run`. It also provides `ic-crates-blob-worker`,
`ic_crates::run_blob_worker` and `ic_crates::preflight_blob_worker` for explicit
composition with maintained Blob native/browser/verifier tools. The supporting `ic-crates-protocol` package owns
Cargo metadata and Candid contracts.

After publication, install and run with:

```sh
cargo install --locked ic-crates
ic-crates /absolute/path/bridge.json
```

The bridge receives authenticated Cargo uploads, validates their framing and
archive structure, computes the upstream Blob Storage manifest, and retains
original upload jobs before admission or transfer. It commits a version only
after the IC registry authenticates verifier completion and a live blob reference.
Retries preserve the original permission and never restart the provider worker.

An installed/configured registry, Blob Service tenant enrollment, controller
signer, private Cargo token, private spool and an explicitly reviewed Blob Storage
worker are required. The supplied worker consumes an already-admitted permission
with explicit selected tools and signers; Blob owns browser/SDK transfer and
independent verifier journals. See the [worker setup and preflight guide](https://github.com/dragginzgame/ic-crates/blob/main/docs/blob-worker.md). Configure an HTTPS front proxy with bounded ingress/deadlines; the
bridge listens only on loopback. This package does not deploy or fund canisters.

Follow the [configuration, worker and recovery guide](https://github.com/dragginzgame/ic-crates/blob/main/docs/cargo-publish.md)
before running it. Local tests exercise unmodified Cargo publication, sparse
index polling, download and consumer compilation against substitutes. Deployed
IC/provider acceptance remains separate qualification work.

Minimum supported Rust: 1.91.0. MIT license. Native Linux and macOS application;
canister packages are separate components. The former development command
`ic-crates-publisher` is now `ic-crates`.
