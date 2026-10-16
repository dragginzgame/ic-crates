#!/usr/bin/env bash
set -euo pipefail

# Cargo resolves the real graph; jq checks only the two canister runtime owners.
scratch="$(mktemp -d "${TMPDIR:-/tmp}/ic-crates-runtime.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
cargo metadata --offline --locked --format-version 1 > "$scratch/metadata.json"
jq -e '
  .packages as $packages
  | ["ic-memory", "ic-timers"]
  | all(.[]; . as $name | [$packages[] | select(.name == $name)] | length == 1)
' "$scratch/metadata.json" >/dev/null || {
    jq -r '
      .packages as $packages
      | ["ic-memory", "ic-timers"][] as $name
      | [$packages[] | select(.name == $name)] as $selected
      | select($selected | length != 1)
      | "\($name): \($selected | map("\(.version) (\(.source // "local source"))") | join(", "))"
    ' "$scratch/metadata.json" >&2
    echo 'Canister graphs require one Memory identity and one Timers identity.' >&2
    exit 1
}
echo 'One Memory and one Timers identity selected.'
