#!/usr/bin/env bash
# Runs the full workspace test suite plus static checks and, when the Noir
# toolchain is available, the circuit suites.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo test (workspace)"
cargo test --workspace

echo "==> static checks"
bash scripts/check.sh

if command -v nargo >/dev/null 2>&1; then
    echo "==> nargo test (circuit workspace)"
    (cd circuits && nargo test)
    echo "==> circuit test vectors"
    bash scripts/generate-test-vectors.sh
else
    echo "nargo not on PATH; skipping circuit suites (see scripts/check-circuits.sh)." >&2
fi

echo "All tests passed."
