#!/usr/bin/env bash
# Installs the pinned Noir toolchain (nargo) that this repository is
# developed and tested against.
#
# The exact version lives in crates/noir/src/lib.rs (TESTED_NARGO_VERSION)
# and is mirrored in .github/workflows/ci.yml; keep all three in sync. A
# circuit change that needs a newer compiler must bump the pin in all three
# places and re-run the artifact determinism gate (artifacts check).
#
# Idempotent: re-running upgrades the active nargo to the pinned version.
set -euo pipefail

PINNED_NARGO_VERSION="1.0.0-beta.26"

if command -v nargo >/dev/null 2>&1 && [[ "$(nargo --version | head -1)" == *"$PINNED_NARGO_VERSION"* ]]; then
    echo "nargo $PINNED_NARGO_VERSION already installed: $(command -v nargo)"
    exit 0
fi

# noirup installs nargo into ~/.nargo/bin; add it to PATH for this run.
if ! command -v noirup >/dev/null 2>&1; then
    echo "==> installing noirup"
    curl -sL https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
fi
export PATH="$HOME/.nargo/bin:$PATH"

echo "==> installing nargo $PINNED_NARGO_VERSION via noirup"
noirup -v "$PINNED_NARGO_VERSION"

nargo --version
echo
echo "nargo $PINNED_NARGO_VERSION ready."
echo "Add ~/.nargo/bin to PATH (or run scripts/check-circuits.sh to verify)."
