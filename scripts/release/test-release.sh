#!/usr/bin/env bash
set -euo pipefail

# Actual shared runner and consumer metadata in disposable Git/bare-remote repos.
# Native validation and setup are substituted here; make ci qualifies real code.
root="$PWD"
export PATH="$root/.tools/host/bin:$root/.tools/rust/common/bin:$PATH"
# shellcheck source=ci/product-tools.env
source "$root/ci/product-tools.env"
release_editor="$(bash scripts/ci/managed-ic.sh metadata-editor)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/ic-crates-release.XXXXXX")"
complete=false
finish() {
    local status=$?
    [[ "$complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "Release fixture retained: $fixture" >&2; fi
    exit "$status"
}
trap finish EXIT
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES
unset MAKEFLAGS MFLAGS MAKEOVERRIDES GNUMAKEFLAGS MAKEFILES
unset RELEASE_SOURCE RELEASE_VERSION RELEASE_PREVIOUS RELEASE_DATE RELEASE_COMMIT RELEASE_DELIVERY
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir "$fixture/repo"
cp -R scripts make ci apps crates docs .githooks "$fixture/repo/"
cp Cargo.toml Cargo.lock CHANGELOG.md LICENSE rust-toolchain.toml .gitignore "$fixture/repo/"
rm -f "$fixture/repo/docs/release.json"
cat > "$fixture/repo/Makefile" <<'MAKE'
SHARED_TOOLING_ROOT := $(CURDIR)
include make/release.mk
.PHONY: ci deps install-tools tools-check install-validation-tools lint check-secrets release-version release-preflight release-verify \
release-prepare-version release-prepared-check release-files release-commit-check \
release-committed-check release-tagged-check release-push-check
ci:
	@mkdir -p target
	@echo gate >> target/gates
	@test "$${MOCK_GATE_FAILURE:-0}" = 0
deps install-tools tools-check install-validation-tools lint check-secrets:
	@:
release-version:
	@bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml
release-preflight release-verify release-prepare-version release-prepared-check \
release-files release-commit-check release-committed-check release-tagged-check release-push-check:
	@bash scripts/release/release.sh "$(@:release-%=%)"
MAKE
# Prepare only the fixture's selected pointer to the already-admitted editor.
# The shared store owns executable bytes; this offline setup cannot fetch inputs.
CARGO_NET_OFFLINE=true bash "$fixture/repo/scripts/dev/install-rust-tools.sh" \
    --consumer "$fixture/repo" --package ic-build --version "$IC_CRATES_RELEASE_EDITOR_VERSION" \
    --bin ic-build-release-metadata --features cli --profile debug > "$fixture/editor-selection.log" 2>&1

# Recovery tool admission must not resolve partially prepared live Cargo metadata.
cp "$fixture/repo/Cargo.lock" "$fixture/tool-selection.lock"
printf 'not a lockfile\n' > "$fixture/repo/Cargo.lock"
fixture_editor="$(bash "$fixture/repo/scripts/ci/managed-ic.sh" metadata-editor)"
"$fixture_editor" --version > "$fixture/recovery-editor-version.log"
cp "$fixture/tool-selection.lock" "$fixture/repo/Cargo.lock"

# Qualify the consumer's grouped formatting contract before any Git fixture effect.
cp "$fixture/repo/Cargo.toml" "$fixture/grouped.manifest"
(cd "$fixture/repo" && cargo sort --workspace --grouped --check)
perl -0pi -e 's{(flate2 = [^\n]+\n)(getrandom = [^\n]+\n)}{$2$1} or die "fixture needs two production dependencies\n"' "$fixture/repo/Cargo.toml"
if (cd "$fixture/repo" && cargo sort --workspace --grouped --check) > "$fixture/unsorted-refusal.log" 2>&1; then
    echo 'Grouped formatting accepted unsorted production dependencies.' >&2; exit 1
fi
(cd "$fixture/repo" && cargo sort --workspace --grouped)
cmp "$fixture/grouped.manifest" "$fixture/repo/Cargo.toml"

git init -q "$fixture/repo"
git -C "$fixture/repo" symbolic-ref HEAD refs/heads/main
git -C "$fixture/repo" config user.name 'Release fixture'
git -C "$fixture/repo" config user.email 'release-fixture@example.invalid'
git -C "$fixture/repo" config commit.gpgsign false
git -C "$fixture/repo" config tag.gpgsign false
git init --bare -q "$fixture/remote.git"
git -C "$fixture/repo" remote add origin "$fixture/remote.git"
cd "$fixture/repo"
# The disposable fixture owns its release draft even on finalized-only source.
previous="$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)"
kind='patch'
candidate="$(bash scripts/ci/next-release-version.sh "$previous" "$kind")"
printf '## [%s] - 2020-01-01\n\n- Previous fixture release.\n\n## [Draft]\n\n- Archived fixture wording.  \t' \
    "$previous" > "$fixture/historical-notes"
printf '# Changelog\n\n## [%s]\n\n- Exercise isolated release recovery.\n\n' \
    "$candidate" > CHANGELOG.md
cat "$fixture/historical-notes" >> CHANGELOG.md
# This index is disposable. Never stage or mutate the consumer checkout's index.
git add .
git commit -qm 'Fixture source'
git push -q -u origin main
source="$(git rev-parse HEAD)"
date="$(date -u +%Y-%m-%d)"
export RELEASE_SOURCE="$source" RELEASE_PREVIOUS="$previous" RELEASE_VERSION="$candidate" RELEASE_DATE="$date"
cp Cargo.lock "$fixture/original.lock"

# Fixture-owned notes do not weaken admission of an actual release's notes.
cp CHANGELOG.md "$fixture/valid-notes"
printf '# Changelog\n' > CHANGELOG.md
if bash scripts/release/release.sh preflight > "$fixture/missing-notes-refusal.log" 2>&1; then
    echo 'Preflight accepted missing actual release notes.' >&2; exit 1
fi
[[ "$(git rev-parse HEAD)" == "$source" ]]
[[ "$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)" == "$previous" ]]
cmp "$fixture/original.lock" Cargo.lock
cp "$fixture/valid-notes" CHANGELOG.md

# An active unnumbered draft still refuses before any release effect.
printf '# Changelog\n\n## [%s]\n\n## [Draft]\n\n' "$candidate" > CHANGELOG.md
cat "$fixture/historical-notes" >> CHANGELOG.md
if bash scripts/release/release.sh preflight > "$fixture/active-draft-refusal.log" 2>&1; then
    echo 'Preflight accepted an active unnumbered draft.' >&2; exit 1
fi
[[ "$(git rev-parse HEAD)" == "$source" ]]
[[ "$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)" == "$previous" ]]
cmp "$fixture/original.lock" Cargo.lock
cp "$fixture/valid-notes" CHANGELOG.md

printf 'unrelated\n' > unrelated.txt
git add unrelated.txt
if bash scripts/release/release.sh preflight > "$fixture/refusal.log" 2>&1; then
    echo 'Preflight accepted unrelated staged source.' >&2; exit 1
fi
git reset -q HEAD -- unrelated.txt
rm unrelated.txt
unset RELEASE_SOURCE RELEASE_PREVIOUS RELEASE_VERSION RELEASE_DATE

if MOCK_GATE_FAILURE=1 make --no-print-directory "release-$kind" > "$fixture/failed-gate.log" 2>&1; then
    echo 'A failed validation gate released metadata.' >&2; exit 1
fi
[[ "$(git rev-parse HEAD)" == "$source" ]]
[[ "$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)" == "$previous" ]]
if git rev-parse --verify "refs/tags/v$candidate" >/dev/null 2>&1; then
    echo 'Failed validation created a tag.' >&2; exit 1
fi
# Fail the consumer receipt inside shared metadata preparation. All Cargo inputs
# must roll back before explicit recovery of the original saved attempt.
real_perl="$(command -v perl)"
mkdir "$fixture/bin"
cat > "$fixture/bin/perl" <<'PERL_WRAPPER'
#!/usr/bin/env bash
if [[ "${MOCK_RECEIPT_FAILURE:-0}" == 1 && "${1:-}" == scripts/release/metadata.pl && "${2:-}" == receipt ]]; then
    exit 1
fi
exec "$REAL_FIXTURE_PERL" "$@"
PERL_WRAPPER
chmod +x "$fixture/bin/perl"
if PATH="$fixture/bin:$PATH" REAL_FIXTURE_PERL="$real_perl" MOCK_RECEIPT_FAILURE=1 \
    make --no-print-directory "release-$kind" > "$fixture/receipt-failure.log" 2>&1; then
    echo 'Receipt failure unexpectedly completed a release.' >&2; exit 1
fi
[[ "$(git rev-parse HEAD)" == "$source" ]]
[[ "$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)" == "$previous" ]]
cmp "$fixture/original.lock" Cargo.lock
make --no-print-directory release-resume VERSION="$candidate" > "$fixture/release.log" 2>&1
commit="$(git rev-parse HEAD)"
[[ "$commit" != "$source" ]]
[[ "$(git log -1 --format=%P)" == "$source" ]]
[[ "$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)" == "$candidate" ]]
printf '# Changelog\n\n## [%s] - %s\n\n- Exercise isolated release recovery.\n\n' \
    "$candidate" "$date" > "$fixture/expected-notes"
cat "$fixture/historical-notes" >> "$fixture/expected-notes"
cmp "$fixture/expected-notes" CHANGELOG.md
bash scripts/ci/check-release-tag.sh "$commit" "$candidate"
[[ "$(git --git-dir="$fixture/remote.git" rev-parse refs/heads/main)" == "$commit" ]]
[[ "$(git --git-dir="$fixture/remote.git" rev-parse "refs/tags/v$candidate^{commit}")" == "$commit" ]]
perl scripts/release/metadata.pl check-commit "$commit" "$source" "$candidate" "$date"
"$release_editor" --max-input-bytes "$IC_CRATES_RELEASE_METADATA_MAX_BYTES" lock Cargo.lock "$candidate" "$previous" \
    ic-crates-protocol canister_ic_crates_registry canister_ic_crates_blob ic-crates ic-crates-tests > "$fixture/restored.lock"
cmp "$fixture/original.lock" "$fixture/restored.lock"
make --no-print-directory release-resume VERSION="$candidate" > "$fixture/resume.log" 2>&1
[[ "$(git rev-parse HEAD)" == "$commit" ]]

# Selected-source checking must ignore a differently ordered live manifest.
cp Cargo.toml "$fixture/released.manifest"
perl -0pi -e 's{(flate2 = [^\n]+\n)(getrandom = [^\n]+\n)}{$2$1} or die "fixture needs two production dependencies\n"' Cargo.toml
cp Cargo.toml "$fixture/unsorted-live.manifest"
RELEASE_COMMIT="$commit" RELEASE_VERSION="$candidate" RELEASE_DATE="$date" \
    bash scripts/ci/cargo-release-metadata.sh \
    check ic-crates-protocol --manifest-order-grouped > "$fixture/committed-source.log" 2>&1
cmp "$fixture/unsorted-live.manifest" Cargo.toml
cp "$fixture/released.manifest" Cargo.toml

# Metadata edits must not pass the retained source-bound receipt.
printf '\n' >> Cargo.toml
if perl scripts/release/metadata.pl check "$source" "$candidate" "$date" > "$fixture/receipt-refusal.log" 2>&1; then
    echo 'Receipt accepted changed metadata.' >&2; exit 1
fi
git restore Cargo.toml
complete=true
echo 'Release fixture passed: historical-note preservation, active-draft refusal, shared metadata/editor, grouped selected-source checks, gate/receipt recovery and exact local delivery/replay.'
