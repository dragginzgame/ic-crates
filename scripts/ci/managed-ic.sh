#!/usr/bin/env bash
set -euo pipefail

# Application artifacts and assertions are local; Canic/Testkit retain their owners.
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
root="$PWD"
export PATH="$root/.tools/host/bin:$root/.tools/ic/bin:$root/.tools/rust/common/bin:$PATH"
export CARGO_TARGET_DIR="$root/target"
canic_version="$(yq -p toml -o json '.' Cargo.lock | jq -er \
    '[.package[] | select(.name == "canic" and .source == "registry+https://github.com/rust-lang/crates.io-index")] | if length == 1 then .[0].version else error("ambiguous Canic source") end')"
canic_tool() {
    bash scripts/dev/install-rust-tools.sh --consumer "$root" --package canic-cli \
        --version "$canic_version" --bin canic --profile release "$@"
}
server_tool() {
    bash scripts/dev/install-rust-tools.sh --consumer "$root" --package ic-testkit \
        --lockfile Cargo.lock --bin ic-testkit-server --profile release "$@"
}
case "${1:-}" in
    install)
        canic_tool
        server="$(server_tool)"
        "$server" setup --directory "$root/.tools/ic-testkit-server"
        ;;
    check)
        canic_tool --check
        server="$(server_tool --check)"
        "$server" check --directory "$root/.tools/ic-testkit-server"
        ;;
    build)
        canic="$(canic_tool --check)"
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
    *) echo 'Usage: managed-ic.sh install|check|build|test' >&2; exit 2 ;;
esac
