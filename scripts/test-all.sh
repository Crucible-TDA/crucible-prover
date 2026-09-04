#!/usr/bin/env bash
# Runs the full workspace test suite plus static checks.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo test (workspace)"
cargo test --workspace

echo "==> static checks"
bash scripts/check.sh

echo "All tests passed."
