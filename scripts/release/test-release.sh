#!/usr/bin/env bash
set -euo pipefail

# Actual shared runner and consumer metadata in disposable Git/bare-remote repos.
# Native validation and setup are substituted here; make ci qualifies real code.
root="$PWD"
export PATH="$root/.tools/host/bin:$root/.tools/rust/common/bin:$PATH"
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
git init -q "$fixture/repo"
git -C "$fixture/repo" symbolic-ref HEAD refs/heads/main
git -C "$fixture/repo" config user.name 'Release fixture'
git -C "$fixture/repo" config user.email 'release-fixture@example.invalid'
git -C "$fixture/repo" config commit.gpgsign false
git -C "$fixture/repo" config tag.gpgsign false
git init --bare -q "$fixture/remote.git"
git -C "$fixture/repo" remote add origin "$fixture/remote.git"
cd "$fixture/repo"
# This index is disposable. Never stage or mutate the consumer checkout's index.
git add .
git commit -qm 'Fixture source'
git push -q -u origin main
source="$(git rev-parse HEAD)"
previous="$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)"
candidate="$(bash scripts/ci/next-release-version.sh "$previous" patch)"
date="$(date -u +%Y-%m-%d)"
export RELEASE_SOURCE="$source" RELEASE_PREVIOUS="$previous" RELEASE_VERSION="$candidate" RELEASE_DATE="$date"
cp Cargo.lock "$fixture/original.lock"

printf 'unrelated\n' > unrelated.txt
git add unrelated.txt
if bash scripts/release/release.sh preflight > "$fixture/refusal.log" 2>&1; then
    echo 'Preflight accepted unrelated staged source.' >&2; exit 1
fi
git reset -q HEAD -- unrelated.txt
rm unrelated.txt
unset RELEASE_SOURCE RELEASE_PREVIOUS RELEASE_VERSION RELEASE_DATE

if MOCK_GATE_FAILURE=1 make --no-print-directory release-patch > "$fixture/failed-gate.log" 2>&1; then
    echo 'A failed validation gate released metadata.' >&2; exit 1
fi
[[ "$(git rev-parse HEAD)" == "$source" ]]
[[ "$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)" == "$previous" ]]
if git rev-parse --verify "refs/tags/v$candidate" >/dev/null 2>&1; then
    echo 'Failed validation created a tag.' >&2; exit 1
fi
make --no-print-directory release-patch > "$fixture/release.log" 2>&1
commit="$(git rev-parse HEAD)"
[[ "$commit" != "$source" ]]
[[ "$(git log -1 --format=%P)" == "$source" ]]
[[ "$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)" == "$candidate" ]]
bash scripts/ci/check-release-tag.sh "$commit" "$candidate"
[[ "$(git --git-dir="$fixture/remote.git" rev-parse refs/heads/main)" == "$commit" ]]
[[ "$(git --git-dir="$fixture/remote.git" rev-parse "refs/tags/v$candidate^{commit}")" == "$commit" ]]
perl scripts/release/metadata.pl check-commit "$commit" "$source" "$candidate" "$date"
perl scripts/ci/rewrite-local-lock-versions.pl Cargo.lock "$candidate" "$previous" \
    ic-crates-protocol canister_ic_crates_registry canister_ic_crates_blob ic-crates-publisher > "$fixture/restored.lock"
cmp "$fixture/original.lock" "$fixture/restored.lock"
make --no-print-directory release-resume VERSION="$candidate" > "$fixture/resume.log" 2>&1
[[ "$(git rev-parse HEAD)" == "$commit" ]]

# Metadata edits must not pass the retained source-bound receipt.
printf '\n' >> Cargo.toml
if perl scripts/release/metadata.pl check "$source" "$candidate" "$date" > "$fixture/receipt-refusal.log" 2>&1; then
    echo 'Receipt accepted changed metadata.' >&2; exit 1
fi
git restore Cargo.toml
complete=true
echo 'Release fixture passed: actual metadata/runner, failed gate recovery, exact local delivery and replay.'
