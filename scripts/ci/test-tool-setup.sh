#!/usr/bin/env bash
set -euo pipefail
root="$(cd -P "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/ic-crates-tool-setup.XXXXXX")"
complete=false
finish() {
    local status=$?
    [[ "$complete" == true || "$status" != 0 ]] || status=1
    if [[ "$status" == 0 ]]; then rm -rf "$fixture"
    else printf 'Tool setup fixture retained: %s\n' "$fixture" >&2; fi
    exit "$status"
}
trap finish EXIT
mkdir -p "$fixture/consumer/scripts/ci" "$fixture/consumer/scripts/dev" "$fixture/bin" "$fixture/cache"
cp "$root/Makefile" "$root/Cargo.lock" "$fixture/consumer/"
cp -R "$root/make" "$fixture/consumer/"
cp "$root/scripts/ci/"{managed-ic,check-make-execution}.sh "$fixture/consumer/scripts/ci/"
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
        printf '%s\n' '{"packages":[{"name":"ic-crates-tests","id":"harness"},{"name":"ic-testkit","id":"kit","version":"0.35.1","source":"registry+https://github.com/rust-lang/crates.io-index"}],"resolve":{"nodes":[{"id":"harness","deps":[{"name":"ic_testkit","pkg":"kit"}]}]}}' ;;
    *) exit 97 ;;
esac
CARGO
cat > "$fixture/consumer/scripts/dev/install-rust-tools.sh" <<'INSTALL'
#!/usr/bin/env bash
set -euo pipefail
case " $* " in
    *' --package canic-cli '*) printf 'canic\n' >> "$SETUP_TRACE" ;;
    *' --package ic-testkit '*)
        printf 'testkit\n' >> "$SETUP_TRACE"
        [[ " $* " == *' --version 0.35.1 '* ]] || exit 98
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
    env -u MAKEFLAGS -u MFLAGS -u MAKELEVEL -u MAKEFILES \
        make --no-print-directory -C "$fixture/consumer" -j4 "$@"
}
run_make install-tools > "$fixture/setup.log" 2>&1
printf '%s\n' 'fetch --locked' canic 'metadata --offline --locked --format-version 1' testkit 'server setup' > "$fixture/expected"
cmp "$fixture/trace" "$fixture/expected"
rm "$CARGO_HOME/prepared"
: > "$fixture/trace"
if SETUP_FETCH_FAILURE=1 run_make install-tools > "$fixture/fetch-refusal.log" 2>&1; then exit 1; fi
printf '%s\n' 'fetch --locked' > "$fixture/expected"
cmp "$fixture/trace" "$fixture/expected"
: > "$fixture/trace"
if run_make tools-check > "$fixture/offline-refusal.log" 2>&1; then exit 1; fi
printf '%s\n' canic 'metadata --offline --locked --format-version 1' > "$fixture/expected"
cmp "$fixture/trace" "$fixture/expected"
status=0
(cd "$fixture/consumer"; bash scripts/ci/managed-ic.sh check) > "$fixture/metadata-refusal.log" 2>&1 || status=$?
[[ "$status" == 41 ]] || exit 1
cmp "$root/Cargo.lock" "$fixture/consumer/Cargo.lock"
complete=true
printf 'Cold setup, fetch failure and offline selection refusal passed\n'
