#!/usr/bin/env bash
# Verifies the Barretenberg backend (bb) is available and reports the version.
# Live proving batches gate on this before running bb-driven integration
# tests. The pinned version lives in crates/ultrahonk/src/lib.rs
# (TESTED_BB_VERSION); CI installs exactly that version.
set -euo pipefail
cd "$(dirname "$0")/.."

if command -v bb >/dev/null 2>&1; then
    echo "bb found: $(bb --version)"
    exit 0
fi

echo "bb is not installed on PATH." >&2
echo "Install it with:" >&2
echo "  curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/next/barretenberg/bbup/install | bash" >&2
echo "  bbup -v 6.0.0-nightly.20260903 --no-modify-path   # keep in sync with TESTED_BB_VERSION" >&2
if [[ "${CI:-}" == "true" ]]; then
    exit 1
fi
echo "Not in CI; treating missing toolchain as skippable." >&2
exit 0
