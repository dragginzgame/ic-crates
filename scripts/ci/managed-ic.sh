#!/usr/bin/env bash
set -euo pipefail

# Application artifacts and assertions are local; Canic/Testkit retain their owners.
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
root="$PWD"
export PATH="$root/.tools/host/bin:$root/.tools/ic/bin:$root/.tools/rust/common/bin:$PATH"
export CARGO_TARGET_DIR="$root/target"
# shellcheck source=ci/product-tools.env
source "$root/ci/product-tools.env"
canic_tool() {
    local canic_version
    canic_version="$(yq -p toml -o json '.' Cargo.lock | jq -er \
        '[.package[] | select(.name == "canic" and .source == "registry+https://github.com/rust-lang/crates.io-index")] | if length == 1 then .[0].version else error("ambiguous Canic source") end')"
    bash scripts/dev/install-rust-tools.sh --consumer "$root" --package canic-cli \
        --version "$canic_version" --bin canic --profile release "$@"
}
candid_tool() {
    bash scripts/dev/install-rust-tools.sh --consumer "$root" --package candid-extractor \
        --version "$IC_CRATES_CANDID_EXTRACTOR_VERSION" --bin candid-extractor --profile release "$@"
}
release_metadata_tool() {
    bash scripts/dev/install-rust-tools.sh --consumer "$root" --package ic-build \
        --version "$IC_CRATES_RELEASE_EDITOR_VERSION" --bin ic-build-release-metadata \
        --features cli --profile debug "$@"
}
server_tool() {
    # Select the harness's direct Testkit edge, not every upstream SDK's copy.
    # Cargo owns version resolution; no downstream semver matcher is needed.
    local version
    version="$(cargo metadata --offline --locked --format-version 1 | jq -er '
        . as $metadata
        | [.packages[] | select(.name == "ic-crates-tests")]
        | if length == 1 then .[0].id else error("ambiguous test harness") end
        | . as $harness
        | [$metadata.resolve.nodes[] | select(.id == $harness) | .deps[] | select(.name == "ic_testkit") | .pkg]
        | if length == 1 then .[0] else error("ambiguous harness Testkit edge") end
        | . as $selected
        | $metadata.packages[] | select(.id == $selected)
        | if .source == "registry+https://github.com/rust-lang/crates.io-index"
          then .version else error("Testkit tool requires released source") end')"
    bash scripts/dev/install-rust-tools.sh --consumer "$root" \
        --package ic-testkit --version "$version" --bin ic-testkit-server \
        --features cli --profile release "$@"
}
case "${1:-}" in
    install)
        canic_tool
        candid_tool
        release_metadata_tool
        server="$(server_tool)"
        "$server" setup --directory "$root/.tools/ic-testkit-server"
        ;;
    check)
        canic_tool --check
        candid_tool --check
        release_metadata_tool --check
        server="$(server_tool --check)"
        "$server" check --directory "$root/.tools/ic-testkit-server"
        ;;
    build)
        canic="$(canic_tool --check)"
        extractor="$(candid_tool --check)"
        export PATH="${extractor%/*}:$PATH"
        mkdir -p target/managed-builds
        attempt="$(mktemp -d "$root/target/managed-builds/attempt.XXXXXX")"
        cp apps/ic_crates/canic.toml "$attempt/canic.toml"
        "$canic" build ic_crates --workspace "$root" --config "$root/apps/ic_crates/canic.toml" \
            --icp-root "$root/apps/ic_crates" --profile fast --json > "$attempt/build.json" 2> "$attempt/build.stderr"
        cmp "$attempt/canic.toml" apps/ic_crates/canic.toml
        jq -e 'select(.completed == true and .event == "build_completed" and .role == null)' "$attempt/build.json" > /dev/null
        # Only a completed result becomes current; failed attempts remain untouched.
        cp "$attempt/build.json" target/managed-build.json
        echo "Managed build evidence retained: $attempt"
        ;;
    test)
        server="$(server_tool --check)"
        mkdir -p target/managed-tests
        attempt="$(mktemp -d "$root/target/managed-tests/attempt.XXXXXX")"
        export IC_CRATES_BUILD_RESULT="$root/target/managed-build.json"
        "$server" run --directory "$root/.tools/ic-testkit-server" --ttl 600 \
            --server-stdout "$attempt/server.stdout" --server-stderr "$attempt/server.stderr" -- \
            cargo test --offline --locked -p ic-crates-tests --lib -- --ignored --nocapture --test-threads=1
        echo "Managed test evidence retained: $attempt"
        ;;
    metadata-editor)
        release_metadata_tool --check
        ;;
    *) echo 'Usage: managed-ic.sh install|check|build|test|metadata-editor' >&2; exit 2 ;;
esac
