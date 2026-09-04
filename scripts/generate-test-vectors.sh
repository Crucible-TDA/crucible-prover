#!/usr/bin/env bash
# Executes every committed circuit test vector (circuits/<op>/testdata/Prover.toml)
# through nargo, asserting each witness solves. This is the canary that the
# committed public inputs still match each circuit's current witness layout:
# a renamed param, a widened amount, or a changed commitment scheme fails here.
#
# Requires the nargo toolchain on PATH (see scripts/check-circuits.sh).
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v nargo >/dev/null 2>&1; then
    echo "nargo is not installed on PATH; cannot generate witnesses." >&2
    exit 1
fi

cd circuits

operations=(register deposit merge transfer withdraw)
for op in "${operations[@]}"; do
    vector="testdata/Prover.toml"
    if [[ ! -f "$op/$vector" ]]; then
        echo "SKIP $op (no $vector)" >&2
        continue
    fi
    echo "==> solving witness for $op"
    # --prover-name resolves relative to the package directory; nargo refuses
    # absolute paths outside the package, so run from the package dir.
    (cd "$op" && nargo execute --package "$op" --prover-name testdata/Prover >/dev/null)
done

echo "All circuit test vectors solved."
