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
# shellcheck source=ci/product-tools.env
source "$root/ci/product-tools.env"

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
    bash scripts/ci/cargo-release-metadata.sh check ic-crates-protocol --manifest-order-grouped
}
committed() {
    clean
    bash scripts/ci/cargo-release-metadata.sh check ic-crates-protocol --manifest-order-grouped
    perl scripts/release/metadata.pl check-commit "${RELEASE_COMMIT:?}" \
        "${RELEASE_SOURCE:?}" "${RELEASE_VERSION:?}" "${RELEASE_DATE:?}"
}
# Product context projection only; IC Build owns both candidate renderers.
# shellcheck disable=SC2329 # Callback invoked by the shared metadata transaction.
render_metadata() {
    local original="$1" candidate="$2" context="${2%/*}/workspace.json"
    local manifest="${2%/*}/manifest.json" metadata="${2%/*}/metadata.json"
    local packages=() package
    yq -p toml -o json '.' "$original/Cargo.toml" > "$manifest" || return
    cargo metadata --offline --locked --no-deps --format-version 1 > "$metadata" || return
    jq -L "$root/scripts/ci" --slurpfile manifest "$manifest" \
        --arg previous "$RELEASE_PREVIOUS" '
        include "cargo-workspace-members";
        {manifest: $manifest[0], members: (coordinated_members("ic-crates-protocol"; $previous)
            | map({name, manifest_path}))}' "$metadata" > "$context" || return
    while IFS= read -r package; do packages+=("$package"); done < <(jq -r '.members[].name' "$context")
    "$editor" --max-input-bytes "$IC_CRATES_RELEASE_METADATA_MAX_BYTES" manifest Cargo.toml \
        "$RELEASE_PREVIOUS" "$RELEASE_VERSION" "$context" > "$candidate/Cargo.toml" || return
    "$editor" --max-input-bytes "$IC_CRATES_RELEASE_METADATA_MAX_BYTES" lock "$original/Cargo.lock" \
        "$RELEASE_PREVIOUS" "$RELEASE_VERSION" "${packages[@]}" > "$candidate/Cargo.lock" || return
    notes > "$candidate/CHANGELOG.md"
}

# shellcheck disable=SC2329 # Callback invoked by the shared metadata transaction.
check_preparation() {
    perl scripts/release/metadata.pl receipt "$RELEASE_SOURCE" "$RELEASE_VERSION" "$RELEASE_DATE" || return
    prepared
}

prepare() (
    allowed
    source_check
    [[ "$(version)" == "${RELEASE_PREVIOUS:?}" ]] || fail 'Version differs from saved intent.'
    local editor receipt_backup had_receipt=no complete=false
    editor="$(bash scripts/ci/managed-ic.sh metadata-editor)"
    [[ ! -L docs/release.json && ! -d docs/release.json ]] || fail 'Invalid receipt path.'
    mkdir -p target
    receipt_backup="$(mktemp -d "$root/target/release-receipt.XXXXXX")"
    if [[ -f docs/release.json ]]; then
        cp -p docs/release.json "$receipt_backup/release.json"
        had_receipt=yes
    fi
    # Shared owns rollback of the three Cargo inputs. This product owns only
    # restoration of its optional receipt, which is not a stock Cargo document.
    trap 'status=$?; trap - EXIT;
        if [[ "$complete" != true ]]; then
            if [[ "$had_receipt" == yes ]]; then cp -p "$receipt_backup/release.json" docs/release.json || status=1;
            else rm -f docs/release.json || status=1; fi;
            [[ "$status" != 0 ]] || status=1;
        fi;
        echo "Receipt preparation evidence retained: $receipt_backup" >&2;
        exit "$status"' EXIT
    # shellcheck source=scripts/ci/release-metadata-transaction.sh
    source "$root/scripts/ci/release-metadata-transaction.sh"
    release_metadata_prepare "$root/target/release-preparation.XXXXXX" \
        render_metadata check_preparation Cargo.toml Cargo.lock CHANGELOG.md
    complete=true
)


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
