#!/usr/bin/env bash
set -euo pipefail

# Exercise the real publication adapter with substitute Cargo/registry responses.
# The Git checkout, tag and all upload effects below are disposable local fixtures.
root="$PWD"
export PATH="$root/.tools/host/bin:$root/.tools/rust/common/bin:$PATH"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/ic-crates-publication.XXXXXX")"
complete=false
finish() {
    local status=$?
    if [[ "$complete" == true && "$status" == 0 ]]; then rm -rf "$fixture";
    else echo "Publication fixture retained: $fixture" >&2; fi
    exit "$status"
}
trap finish EXIT
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
mkdir -p "$fixture/repo/scripts/release" "$fixture/repo/scripts/ci" "$fixture/repo/docs" "$fixture/bin"
cp scripts/release/publish.sh "$fixture/repo/scripts/release/"
cp scripts/ci/check-release-source.sh scripts/ci/check-release-tag.sh \
    scripts/ci/read-cargo-workspace-version.sh scripts/ci/verify-file-checksum.sh "$fixture/repo/scripts/ci/"
cat > "$fixture/repo/Cargo.toml" <<'TOML'
[workspace.package]
version = "1.2.3"
TOML
printf '/target/\n' > "$fixture/repo/.gitignore"
printf '{"source":"fixture","date":"2026-10-12"}\n' > "$fixture/repo/docs/release.json"
# Receipt structure is qualified by test-release; isolate the batch dispatcher here.
cat > "$fixture/repo/scripts/release/release.sh" <<'RECEIPT'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == committed-check ]]
RECEIPT
cat > "$fixture/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf 'cargo %s\n' "$*" >> "$TEST_STATE/trace"
operation="$1"; shift
case "$operation" in
    locate-project)
        [[ "$*" == '--workspace --message-format plain --manifest-path Cargo.toml' ]]
        printf '%s/Cargo.toml\n' "$PWD"
        ;;
    package)
        [[ "$*" == '--locked --registry crates-io -p ic-crates-protocol -p ic-crates' ]]
        mkdir -p "$CARGO_TARGET_DIR/package"
        for package in ic-crates-protocol ic-crates; do
            printf 'fixture:%s:1.2.3\n' "$package" > "$CARGO_TARGET_DIR/package/$package-1.2.3.crate"
        done
        ;;
    publish)
        [[ "$*" == '--locked --registry crates-io -p ic-crates-protocol' || "$*" == '--locked --registry crates-io -p ic-crates' ]]
        package="${!#}"
        if [[ "$package" == ic-crates ]]; then [[ -f "$TEST_STATE/ic-crates-protocol.present" ]]; fi
        printf '%s\n' "$package" >> "$TEST_STATE/uploads"
        [[ "${TEST_FAIL_UPLOAD_PACKAGE:-}" != "$package" ]] || exit 101
        : > "$TEST_STATE/$package.present"
        if [[ "${TEST_MUTATE_PACKAGE:-}" == "$package" ]]; then printf '\n# source changed\n' >> Cargo.toml; fi
        [[ "${TEST_LOST_REPLY_PACKAGE:-}" != "$package" ]] || exit 101
        ;;
    *) exit 2 ;;
esac
CARGO
cat > "$fixture/repo/scripts/ci/check-crates-io-version.sh" <<'OBSERVER'
#!/usr/bin/env bash
set -euo pipefail
[[ $# == 4 && "$1" == --metadata && "$4" == 1.2.3 ]]
directory="$2" package="$3"
printf 'observe %s %s\n' "$package" "${directory##*/}" >> "$TEST_STATE/trace"
mkdir "$directory"
[[ "${TEST_UNKNOWN_PACKAGE:-}" != "$package" ]] || exit 2
if [[ "${directory##*/}" == "$package-after" && "${TEST_UNKNOWN_AFTER_PACKAGE:-}" == "$package" ]]; then exit 2; fi
[[ -f "$TEST_STATE/$package.present" ]] || exit 1
checksum="$(bash scripts/ci/verify-file-checksum.sh --print sha256 "$CARGO_TARGET_DIR/package/$package-1.2.3.crate")"
if [[ "${TEST_MISMATCH_PACKAGE:-}" == "$package" ]]; then checksum="$(printf '%064d' 0)"; fi
yanked=false
if [[ "${TEST_YANKED_PACKAGE:-}" == "$package" ]]; then yanked=true; fi
printf '{"checksum":"%s","yanked":%s}\n' "$checksum" "$yanked"
OBSERVER
chmod +x "$fixture/bin/cargo"
export PATH="$fixture/bin:$PATH"
git init -q "$fixture/repo"
git -C "$fixture/repo" config user.name 'Publication fixture'
git -C "$fixture/repo" config user.email 'publication-fixture@example.invalid'
git -C "$fixture/repo" config commit.gpgsign false
git -C "$fixture/repo" config tag.gpgsign false
cd "$fixture/repo"
git add .
git commit -qm 'Publication fixture'
git tag -am 'Fixture 1.2.3' v1.2.3 >/dev/null

new_case() {
    export TEST_STATE="$fixture/$1"
    mkdir "$TEST_STATE"
    : > "$TEST_STATE/uploads"
    : > "$TEST_STATE/trace"
}
assert_uploads() {
    local expected="$1" actual
    actual="$(cat "$TEST_STATE/uploads")"
    [[ "$actual" == "$expected" ]] || { echo "Unexpected publication order/count: $actual" >&2; exit 1; }
}
refuse() {
    if env "$@" bash scripts/release/publish.sh > "$TEST_STATE/output" 2>&1; then
        echo 'Publication fixture unexpectedly accepted refusal.' >&2; exit 1
    fi
}

new_case absent
bash scripts/release/publish.sh > "$TEST_STATE/output" 2>&1
assert_uploads $'ic-crates-protocol\nic-crates'
# Both preflight observations must precede either upload.
awk '/^observe .*preflight$/ { checks++ } /^cargo publish/ { if (checks != 2) exit 1 }' "$TEST_STATE/trace"
bash scripts/release/publish.sh >> "$TEST_STATE/output" 2>&1
assert_uploads $'ic-crates-protocol\nic-crates'

new_case protocol-present
: > "$TEST_STATE/ic-crates-protocol.present"
bash scripts/release/publish.sh > "$TEST_STATE/output" 2>&1
assert_uploads ic-crates

new_case unknown
refuse TEST_UNKNOWN_PACKAGE=ic-crates
assert_uploads ''

new_case mismatch
: > "$TEST_STATE/ic-crates.present"
refuse TEST_MISMATCH_PACKAGE=ic-crates
assert_uploads ''

new_case yanked
: > "$TEST_STATE/ic-crates-protocol.present"
refuse TEST_YANKED_PACKAGE=ic-crates-protocol
assert_uploads ''

new_case lost-reply
TEST_LOST_REPLY_PACKAGE=ic-crates-protocol bash scripts/release/publish.sh > "$TEST_STATE/output" 2>&1
assert_uploads $'ic-crates-protocol\nic-crates'

new_case failed-upload
refuse TEST_FAIL_UPLOAD_PACKAGE=ic-crates-protocol
assert_uploads ic-crates-protocol

new_case unknown-after
refuse TEST_UNKNOWN_AFTER_PACKAGE=ic-crates-protocol
assert_uploads ic-crates-protocol

new_case changed-source
refuse TEST_MUTATE_PACKAGE=ic-crates-protocol
assert_uploads ic-crates-protocol
git restore Cargo.toml

new_case dirty-source
printf '\n# uncommitted\n' >> Cargo.toml
refuse
assert_uploads ''
git restore Cargo.toml
complete=true
echo 'Publication fixture passed: exact batch, dependency order, replay, unknown/conflicting observations and interrupted uploads.'
