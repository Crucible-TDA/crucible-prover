#!/usr/bin/env bash
# Verifies the Noir toolchain is available and reports the version.
# Circuit batches gate on this before running nargo-driven integration tests.
set -euo pipefail
cd "$(dirname "$0")/.."

if command -v nargo >/dev/null 2>&1; then
    echo "nargo found: $(nargo --version | head -1)"
    exit 0
fi

echo "nargo is not installed on PATH." >&2
echo "Install it with: curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash && noirup" >&2
if [[ "${CI:-}" == "true" ]]; then
    exit 1
fi
echo "Not in CI; treating missing toolchain as skippable." >&2
exit 0
