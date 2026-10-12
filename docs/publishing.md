# Release and package publication

The publication candidate is `ic-crates-protocol`. Its archive contains only
source, a package README, MIT license and Cargo-generated manifests/lockfile.
The application canisters remain `publish = false`; deploying the registry is
independent of publishing this protocol library to crates.io. The native publisher
package is also private: its [Cargo upload API](cargo-publish.md) publishes client
archives to an installed IC registry, a separate flow from this project's release.

The workspace's initial 0.1.0 is an unpublished development baseline. The first
governed patch candidate is 0.1.1, with one pending numbered changelog entry.
Preparing the repository does not change package versions, create tags, commit,
push or upload packages.

## Preparation

Follow [development setup](development.md), then complete `make ci` and Linux
`make lint check-secrets`. `make publish-dry-run` packages and verifies the protocol
archive without registry mutation; it accepts dirty source for review. The actual
publication command requires an exact clean, tagged source.

## User-operated release

Commit the reviewed implementation and pending notes before selecting one of:

```sh
make release-patch
make release-minor
make release-major
```

All three use the same reviewed Shared Tooling runner and local metadata adapter:
preflight and locked/tool preparation, complete native validation, metadata and
changelog preparation, exact source-bound receipt, scoped commit, annotated tag
and atomic branch/tag delivery to `origin/main`. Only the SemVer component differs.
PR delivery is not enabled by this local adapter. Shared recovery selects saved
intent before starting another release; `make release-resume VERSION=X.Y.Z`
finishes only the retained release. Preserve all plans and build/failure artifacts.

The root Cargo manifest owns versions. The adapter updates inherited workspace
packages, the internal protocol dependency, root lockfile, changelog and
`docs/release.json` as one recoverable metadata transaction. It does not resolve
new dependencies, publish a crate, deploy a service or delete build artifacts.
The [shared release contract](releases.md) owns Git effects and interruption recovery.

## User-operated crates.io publication

```sh
make publish
```

This verifies the clean source, exact current release tag and receipt, then uses
the shared registry observer. A confirmed existing version is an effect-free
success; a confirmed missing version reaches `cargo publish --locked` for the
protocol package. An unknown registry observation stops the command. Cargo owns
authentication through its configured credential provider; tokens are never
embedded in the repository or command arguments.

Publication is separate from release completion. After an interrupted upload,
re-run the observer before sending again. A transport error is not evidence of
an absent version. A tag alone does not prove crates.io publication, native macOS
qualification, application deployment or real provider acceptance.
