#!/usr/bin/env bash
# Static checks: formatting, linting, schema validation.
# Fails on the first failing check so CI output points at the cause.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo fmt --check"
cargo fmt --all --check

echo "==> cargo clippy (all targets, warnings denied)"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> schema validation"
python3 scripts/check-schemas.py

echo "All static checks passed."
