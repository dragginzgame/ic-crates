# Release and package publication

The public packages are `ic-crates-protocol` and `ic-crates`. The protocol library
owns Cargo/Candid contracts; `ic-crates` provides the native bridge executable
and `ic_crates::run` library entry point. Both archives contain their own source,
README, MIT license and Cargo-generated manifests/lockfile. Canister packages
remain `publish = false`. The bridge's [Cargo upload API](cargo-publish.md)
publishes client archives to an installed IC registry, a separate flow from
publishing these tools to crates.io.

The recorded release is 0.2.1; pending 0.3.0 includes the worker and dependency
hard cuts. Package versions remain 0.2.1 until the user releases. The selected
Memory/Wire graph is not release-ready; see [current blockers](status/current.md).
Preparing the repository does not change package versions, create tags, commit,
push or upload packages.

## Preparation

For implementation, follow the [shared validation cadence](../DRAGGINZGAME.md#validation-and-evidence)
and [focused development setup](development.md). A user-operated release requires
complete `make ci` and Linux `make lint check-secrets`. `make publish-dry-run` packages and verifies both public
archives without registry mutation; it accepts dirty source for review. Cargo's
batch packager builds the CLI against the exact protocol archive staged in that
invocation, rather than a previously published same-version copy. Normal registry
handling is required; `--locked` preserves external dependency selection. The actual
publication command requires an exact clean, tagged source.

## User-operated release

Commit the reviewed implementation and pending notes before selecting one of:

```sh
make release-patch
make release-minor
make release-major
```

The current 0.3.0 draft requires `release-minor`; later work chooses the component
appropriate to its pending draft.

All three use the same reviewed Shared Tooling runner and local metadata adapter:
preflight and locked/tool preparation, complete native validation, metadata and
changelog preparation, exact source-bound receipt, scoped commit, annotated tag
and atomic branch/tag delivery to `origin/main`. Only the SemVer component differs.
PR delivery is not enabled by this local adapter. Normal release commands start
fresh from current clean committed source; they do not scan/replay historical
plans. `make release-resume VERSION=X.Y.Z` explicitly recovers that saved attempt.
Preserve attributable build/failure evidence and unresolved external obligations.

The root Cargo manifest owns versions. The adapter updates inherited workspace
packages, the internal protocol dependency, root lockfile, changelog and
`docs/release.json` with a source-bound receipt. Shared Tooling drives the
manifest/lock/changelog transaction through the automatically admitted IC Build
editor. Prepared and committed checks use grouped manifest ordering and the
selected source, not a rewritten live checkout. It does not resolve
new dependencies, publish a crate, deploy a service or delete build artifacts.
The [shared release contract](releases.md) owns Git effects and interruption recovery.

## User-operated crates.io publication

```sh
make publish
```

This verifies the clean source, exact current release tag and receipt, then
packages both archives together and uses the shared registry observer for each.
Any unknown, conflicting or yanked version stops the entire batch before upload.
For confirmed missing versions it publishes `ic-crates-protocol` first, then
`ic-crates`, using normal `cargo publish --locked --registry crates-io -p NAME`.
Each package is observed again immediately before upload and after its result.
The CLI is attempted only after the exact protocol checksum is confirmed.
Already-published packages with matching checksums are skipped independently;
resuming a partially completed batch never needs to re-upload its completed
dependency. An uncertain post-upload observation stops further publication and
retains the attempt under `target/publication-observations`. Cargo owns
authentication through its configured credential provider; tokens are never
embedded in the repository or command arguments.

Publication is separate from release completion. After an interrupted upload,
re-run the observer before sending again. A transport error is not evidence of
an absent version. A tag alone does not prove crates.io publication, native macOS
qualification, application deployment or real provider acceptance.

After the user publishes `ic-crates`, users can install the executable with
`cargo install --locked ic-crates` and run `ic-crates /absolute/path/bridge.json`.
