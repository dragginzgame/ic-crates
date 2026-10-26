#!/usr/bin/env bash
set -euo pipefail
root="$(cd -P "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
# shellcheck source=scripts/ci/fixture-lifecycle.sh
source "$root/scripts/ci/fixture-lifecycle.sh"
fixture="$(fixture_create "${TMPDIR:-/tmp}/ic-crates-tool-setup.XXXXXX")"
trap 'fixture_finish "$?" "$fixture" "Tool setup fixture retained"' EXIT
mkdir -p "$fixture/consumer" "$fixture/bin" "$fixture/cache"
bash "$root/scripts/ci/prepare-script-fixture.sh" "$root" "$fixture/consumer" \
    Makefile Cargo.lock ci/product-tools.env scripts/ci/managed-ic.sh \
    make/consumer.mk scripts/ci/run-validation-targets.sh > "$fixture/sources.log"
for helper in host ic; do
    printf '#!/usr/bin/env bash\nexit 0\n' > "$fixture/consumer/scripts/dev/install-$helper-tools.sh"
done
cat > "$fixture/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "$SETUP_TRACE"
case "$*" in
    'fetch --locked') [[ "${SETUP_FETCH_FAILURE:-0}" == 0 ]] || exit 23; touch "$CARGO_HOME/prepared" ;;
    'metadata --offline --locked --format-version 1')
        [[ -f "$CARGO_HOME/prepared" ]] || { echo 'fixture cache is empty' >&2; exit 41; }
        case "${SETUP_METADATA_MODE:-valid}" in
            invalid) printf '{invalid\n'; exit 0 ;;
            missing-harness) printf '%s\n' '{"packages":[]}'; exit 0 ;;
        esac
        printf '%s\n' '{"packages":[{"name":"ic-crates-tests","id":"harness"},{"name":"ic-testkit","id":"kit","version":"0.35.1","source":"registry+https://github.com/rust-lang/crates.io-index"},{"name":"ic-testkit","id":"other-kit","version":"0.33.1"}],"resolve":{"nodes":[{"id":"harness","deps":[{"name":"ic_testkit","pkg":"kit"}]}]}}'
        [[ "${SETUP_METADATA_MODE:-valid}" != valid-failure ]] || exit 42 ;;
    *) exit 97 ;;
esac
CARGO
cat > "$fixture/consumer/scripts/dev/install-rust-tools.sh" <<'INSTALL'
#!/usr/bin/env bash
set -euo pipefail
case " $* " in
    *' --package canic-cli '*) printf 'canic\n' >> "$SETUP_TRACE" ;;
    *' --package candid-extractor '*) printf 'extractor\n' >> "$SETUP_TRACE" ;;
    *' --package ic-build '*) printf 'editor\n' >> "$SETUP_TRACE" ;;
    *' --package ic-testkit '*)
        printf 'testkit\n' >> "$SETUP_TRACE"
        [[ " $* " == *' --version 0.35.1 '* && " $* " == *' --features cli '* ]] || exit 98
        printf '%s\n' "$SETUP_SERVER" ;;
esac
INSTALL
cat > "$fixture/bin/server" <<'SERVER'
#!/usr/bin/env bash
set -euo pipefail
printf 'server %s\n' "$1" >> "$SETUP_TRACE"
SERVER
chmod +x "$fixture/bin/"*
export PATH="$fixture/bin:$root/.tools/host/bin:$PATH" CARGO_HOME="$fixture/cache"
export SETUP_TRACE="$fixture/trace" SETUP_SERVER="$fixture/bin/server"
run_make() {
    env -u MAKEFLAGS -u MFLAGS -u MAKELEVEL -u MAKEFILES -u MAKEOVERRIDES -u GNUMAKEFLAGS \
        make --no-print-directory -C "$fixture/consumer" -j4 "$@"
}
run_make install-tools > "$fixture/setup.log" 2>&1
printf '%s\n' 'fetch --locked' canic extractor editor 'metadata --offline --locked --format-version 1' testkit 'server setup' > "$fixture/expected"
cmp "$fixture/trace" "$fixture/expected"
rm "$CARGO_HOME/prepared"
: > "$fixture/trace"
if SETUP_FETCH_FAILURE=1 run_make install-tools > "$fixture/fetch-refusal.log" 2>&1; then exit 1; fi
printf '%s\n' 'fetch --locked' > "$fixture/expected"
cmp "$fixture/trace" "$fixture/expected"

# The original setup ordering must still fail with this empty project cache.
cp "$fixture/consumer/Makefile" "$fixture/current.Makefile"
perl -0pi -e 's/LOCAL_TOOL_INSTALL_TARGETS := install-ic-tools deps install-app-tools/LOCAL_TOOL_INSTALL_TARGETS := install-ic-tools install-app-tools/ or die "missing ordered setup\n"' "$fixture/consumer/Makefile"
: > "$fixture/trace"
if run_make install-tools > "$fixture/old-order-refusal.log" 2>&1; then exit 1; fi
cp "$fixture/current.Makefile" "$fixture/consumer/Makefile"
printf '%s\n' canic extractor editor 'metadata --offline --locked --format-version 1' > "$fixture/expected"
cmp "$fixture/trace" "$fixture/expected"
: > "$fixture/trace"
if run_make tools-check > "$fixture/offline-refusal.log" 2>&1; then exit 1; fi
cmp "$fixture/trace" "$fixture/expected"

# Preserve Cargo's status, even when it emits valid-looking JSON before failure.
for mode in missing-cache valid-failure invalid missing-harness; do
    [[ "$mode" == missing-cache ]] || touch "$CARGO_HOME/prepared"
    : > "$fixture/trace"
    status=0
    (cd "$fixture/consumer"; SETUP_METADATA_MODE="$mode" bash scripts/ci/managed-ic.sh check) > "$fixture/$mode.log" 2>&1 || status=$?
    [[ "$status" != 0 ]]
    if [[ "$mode" == missing-cache ]]; then [[ "$status" == 41 ]]; fi
    if [[ "$mode" == valid-failure ]]; then [[ "$status" == 42 ]]; fi
    cmp "$fixture/trace" "$fixture/expected"
done
cmp "$root/Cargo.lock" "$fixture/consumer/Cargo.lock"

# Use the actual CI recipe/runner, substituting only child target effects.
export SETUP_REAL_MAKE SETUP_CI_TRACE="$fixture/ci-trace"
SETUP_REAL_MAKE="$(command -v make)"
cat > "$fixture/bin/make" <<'MAKE'
#!/usr/bin/env bash
set -euo pipefail
for argument in "$@"; do
    if [[ "$argument" == -- ]]; then
        goal="${!#}"
        printf '%s\n' "$goal" >> "$SETUP_CI_TRACE"
        [[ "$goal" != "${SETUP_CI_FAILURE:-}" ]] || exit 43
        exit 0
    fi
done
exec "$SETUP_REAL_MAKE" "$@"
MAKE
chmod +x "$fixture/bin/make"
export VALIDATION_LOG_DIR="$fixture/ci-logs" VALIDATION_FAILURE_LOG_DIR="$fixture/ci-failures"
: > "$SETUP_CI_TRACE"
run_make ci > "$fixture/ci.log" 2>&1
awk '
    $0 == "clippy" { lint = 1 }
    $0 == "check" || $0 == "test" { if (!lint) exit 1; covered++ }
    END { if (covered != 2) exit 1 }
' "$SETUP_CI_TRACE"
: > "$SETUP_CI_TRACE"
if SETUP_CI_FAILURE=clippy run_make ci > "$fixture/clippy-refusal.log" 2>&1; then exit 1; fi
[[ "$(tail -n 1 "$SETUP_CI_TRACE")" == clippy ]]
if grep -Ex 'check|test|docs-check|msrv-check|package-check|managed-build|test-ic' "$SETUP_CI_TRACE"; then exit 1; fi
cmp "$root/Cargo.lock" "$fixture/consumer/Cargo.lock"
fixture_complete=true
printf 'Cold setup, failed observation and early fail-closed Clippy dispatch passed\n'
