#!/usr/bin/env bash
set -euo pipefail

# Explicit package publication, separate from the shared release workflow.
script="${BASH_SOURCE[0]}"
[[ "$script" == /* ]] || script="$PWD/$script"
case "$script" in *$'\n'*|*$'\r'*) exit 2 ;; esac
cd "${script%/*}/../.."
root="$PWD"
export CARGO_TARGET_DIR="$root/target"
export PATH="$root/.tools/host/bin:$root/.tools/ic/bin:$root/.tools/rust/common/bin:$PATH"
bash scripts/ci/check-release-source.sh
version="$(bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml)"
commit="$(git rev-parse HEAD)"
[[ -f docs/release.json && ! -L docs/release.json ]] || { echo 'A source-bound release receipt is required.' >&2; exit 1; }
RELEASE_COMMIT="$commit" RELEASE_SOURCE="$(jq -er .source docs/release.json)" \
    RELEASE_VERSION="$version" RELEASE_DATE="$(jq -er .date docs/release.json)" \
    bash scripts/release/release.sh committed-check
bash scripts/ci/check-release-tag.sh "$commit" "$version"
cargo package --offline --locked -p ic-crates-protocol
archive="$root/target/package/ic-crates-protocol-$version.crate"
checksum="$(bash scripts/ci/verify-file-checksum.sh --print sha256 "$archive")"
bash scripts/ci/check-release-source.sh
[[ "$(git rev-parse HEAD)" == "$commit" ]] || { echo 'Source changed during packaging.' >&2; exit 1; }
mkdir -p target/publication-observations
attempt="$(mktemp -d "$root/target/publication-observations/attempt.XXXXXX")"
status=0
bash scripts/ci/check-crates-io-version.sh --metadata "$attempt/registry" \
    ic-crates-protocol "$version" > "$attempt/metadata.json" || status=$?
case "$status" in
    0)
        jq -e --arg checksum "$checksum" 'select(.checksum == $checksum and .yanked == false)' \
            "$attempt/metadata.json" >/dev/null || { echo 'Existing publication differs or is yanked.' >&2; exit 1; }
        echo "ic-crates-protocol $version is already published with the same archive."
        ;;
    1)
        bash scripts/ci/check-release-source.sh
        [[ "$(git rev-parse HEAD)" == "$commit" ]] || { echo 'Source changed during registry observation.' >&2; exit 1; }
        cargo publish --locked --registry crates-io -p ic-crates-protocol
        ;;
    *) echo "Registry observation unknown; no publication dispatched. Evidence: $attempt" >&2; exit 1 ;;
esac
