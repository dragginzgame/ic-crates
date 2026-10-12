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
packages=(ic-crates-protocol ic-crates)
# Cargo's batch packager verifies the CLI against this exact protocol archive.
cargo package --locked --registry crates-io -p ic-crates-protocol -p ic-crates
checksums=()
for package in "${packages[@]}"; do
    archive="$root/target/package/$package-$version.crate"
    checksums+=("$(bash scripts/ci/verify-file-checksum.sh --print sha256 "$archive")")
done
bash scripts/ci/check-release-source.sh
[[ "$(git rev-parse HEAD)" == "$commit" ]] || { echo 'Source changed during packaging.' >&2; exit 1; }
mkdir -p target/publication-observations
attempt="$(mktemp -d "$root/target/publication-observations/attempt.XXXXXX")"
assert_source() {
    bash scripts/ci/check-release-source.sh
    [[ "$(git rev-parse HEAD)" == "$commit" ]] || { echo 'Source changed during publication.' >&2; exit 1; }
}
observe() {
    local index="$1" phase="$2" status=0 package metadata
    package="${packages[$index]}"
    metadata="$attempt/$package-$phase.json"
    bash scripts/ci/check-crates-io-version.sh --metadata "$attempt/$package-$phase" \
        "$package" "$version" > "$metadata" || status=$?
    case "$status" in
        0)
            jq -e --arg checksum "${checksums[$index]}" \
                'select(.checksum == $checksum and .yanked == false)' "$metadata" >/dev/null || {
                echo "Existing $package $version differs or is yanked. Evidence: $attempt" >&2; exit 1;
            }
            ;;
        1) ;;
        *) echo "Registry observation unknown for $package; no further upload. Evidence: $attempt" >&2; exit 1 ;;
    esac
    observation="$status"
}
# Refuse the entire batch before an upload if either package conflicts or is unknown.
for index in "${!packages[@]}"; do observe "$index" preflight; done
for index in "${!packages[@]}"; do
    package="${packages[$index]}"
    assert_source
    observe "$index" before
    if [[ "$observation" == 0 ]]; then
        echo "$package $version is already published with the same archive."
        continue
    fi
    assert_source
    printf '%s\n' "$commit $package $version ${checksums[$index]}" > "$attempt/$package-upload-intent"
    upload_status=0
    cargo publish --locked --registry crates-io -p "$package" > "$attempt/$package-upload.log" 2>&1 || upload_status=$?
    printf '%s\n' "$upload_status" > "$attempt/$package-upload.exit"
    cat "$attempt/$package-upload.log"
    assert_source
    # Even an apparently successful upload must be observed before the dependent CLI.
    observe "$index" after
    [[ "$observation" == 0 ]] || {
        echo "Publication unconfirmed for $package; preserve this attempt and inspect before resuming: $attempt" >&2; exit 1;
    }
    echo "$package $version publication confirmed."
done
assert_source
