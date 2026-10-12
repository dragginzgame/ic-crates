#!/usr/bin/env bash
set -euo pipefail

# Only local metadata and validation. The shared runner owns every Git effect.
script="${BASH_SOURCE[0]}"
[[ "$script" == /* ]] || script="$PWD/$script"
case "$script" in *$'\n'*|*$'\r'*) echo 'Invalid release directory.' >&2; exit 2 ;; esac
cd "${script%/*}/../.."
root="$PWD"
export CARGO_TARGET_DIR="$root/target"
export PATH="$root/.tools/host/bin:$root/.tools/ic/bin:$root/.tools/rust/common/bin:$PATH"
files=(Cargo.toml Cargo.lock CHANGELOG.md docs/release.json)
packages=(ic-crates-protocol canister_ic_crates_registry canister_ic_crates_blob ic-crates)
release_backup=''
release_had_receipt=no

fail() { echo "Release metadata refused: $*" >&2; exit 1; }
version() { bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml; }
direct() { [[ "${RELEASE_DELIVERY:-direct}" == direct ]] || fail 'Only direct delivery is configured.'; }
clean() { bash scripts/ci/check-release-source.sh; }
allowed() {
    local args=() file
    for file in "${files[@]}"; do args+=(--allow "$file"); done
    bash scripts/ci/check-release-source.sh "${args[@]}"
}
notes() {
    awk -v version="${RELEASE_VERSION:?}" -v previous="${RELEASE_PREVIOUS:?}" \
        -v date="${RELEASE_DATE:?}" -v allow_finalized=1 \
        -f scripts/ci/finalize-release-changelog.awk CHANGELOG.md
}
source_check() {
    [[ "$(git rev-parse HEAD)" == "${RELEASE_SOURCE:?}" ]] || fail 'Source changed.'
}
prepared() {
    allowed
    source_check
    [[ "$(version)" == "${RELEASE_VERSION:?}" ]] || fail 'Prepared version differs.'
    perl scripts/release/metadata.pl check "$RELEASE_SOURCE" "$RELEASE_VERSION" "${RELEASE_DATE:?}"
    cargo metadata --offline --locked --no-deps --format-version 1 >/dev/null
}
committed() {
    clean
    perl scripts/release/metadata.pl check-commit "${RELEASE_COMMIT:?}" \
        "${RELEASE_SOURCE:?}" "${RELEASE_VERSION:?}" "${RELEASE_DATE:?}"
}
prepare() {
    allowed
    source_check
    local current lock_version
    current="$(version)"
    [[ "$current" == "${RELEASE_PREVIOUS:?}" || "$current" == "${RELEASE_VERSION:?}" ]] || fail 'Version differs from saved intent.'
    mkdir -p target
    release_backup="$(mktemp -d "$root/target/release-preparation.XXXXXX")"
    cp Cargo.toml Cargo.lock CHANGELOG.md "$release_backup/"
    if [[ -f docs/release.json ]]; then cp docs/release.json "$release_backup/release.json"; release_had_receipt=yes; fi
    # An interrupted metadata transaction has no provider/Git effects. Keep inputs
    # for diagnosis, and restore on ordinary failures before the shared retry.
    trap 'cp "$release_backup/Cargo.toml" Cargo.toml; cp "$release_backup/Cargo.lock" Cargo.lock; cp "$release_backup/CHANGELOG.md" CHANGELOG.md;
          if [[ "$release_had_receipt" == yes ]]; then cp "$release_backup/release.json" docs/release.json; else rm -f docs/release.json; fi;
          echo "Failed preparation inputs retained: $release_backup" >&2' EXIT
    notes > "$release_backup/finalized.md"
    perl scripts/release/metadata.pl set-version "$RELEASE_PREVIOUS" "$RELEASE_VERSION"
    lock_version="$(yq -p toml -o json '.' Cargo.lock | jq -er \
        '[.package[] | select(.source == null) | .version] | unique | if length == 1 then .[0] else error("mixed local versions") end')"
    if [[ "$lock_version" == "$RELEASE_PREVIOUS" ]]; then
        perl scripts/ci/rewrite-local-lock-versions.pl Cargo.lock "$RELEASE_PREVIOUS" "$RELEASE_VERSION" "${packages[@]}" > "$release_backup/next.lock"
        mv "$release_backup/next.lock" Cargo.lock
    else
        [[ "$lock_version" == "$RELEASE_VERSION" ]] || fail 'Lock version differs from saved intent.'
    fi
    mv "$release_backup/finalized.md" CHANGELOG.md
    cargo sort --workspace
    cargo metadata --offline --locked --no-deps --format-version 1 >/dev/null
    perl scripts/release/metadata.pl receipt "$RELEASE_SOURCE" "$RELEASE_VERSION" "$RELEASE_DATE"
    prepared
    trap - EXIT
    echo "Preparation inputs retained: $release_backup"
}

[[ $# == 1 ]] || fail 'One adapter operation is required.'
case "$1" in
    preflight)
        direct; allowed; source_check
        [[ "$(version)" == "${RELEASE_PREVIOUS:?}" ]] || fail 'Previous version differs.'
        notes >/dev/null
        make --no-print-directory deps
        make --no-print-directory install-tools
        make --no-print-directory tools-check
        if [[ "$(uname -s)" == Linux ]]; then make --no-print-directory install-validation-tools; fi
        source_check; allowed
        ;;
    verify)
        direct
        make --no-print-directory ci
        if [[ "$(uname -s)" == Linux ]]; then make --no-print-directory lint check-secrets; fi
        ;;
    prepare-version) direct; prepare ;;
    prepared-check) prepared ;;
    files) printf '%s\0' "${files[@]}" ;;
    commit-check)
        prepared
        git diff --quiet -- || fail 'Unstaged changes remain.'
        perl scripts/release/metadata.pl index-check
        ;;
    committed-check) committed ;;
    tagged-check|push-check)
        committed
        bash scripts/ci/check-release-tag.sh "$RELEASE_COMMIT" "$RELEASE_VERSION"
        ;;
    *) fail 'Unknown adapter operation.' ;;
esac
