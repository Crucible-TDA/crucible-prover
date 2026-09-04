#!/usr/bin/env bash
# Runs the full workspace test suite plus static checks and, when the Noir
# toolchain is available, the circuit suites.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> cargo test (workspace)"
cargo test --workspace

echo "==> CLI smoke (real binary, real catalog, no toolchain)"
cargo run -q -p crucible-cli -- circuits list >/dev/null
cargo run -q -p crucible-cli -- vectors run
cargo run -q -p crucible-cli -- artifacts check
cargo run -q -p crucible-cli -- artifacts inspect transfer >/dev/null
cargo run -q -p crucible-cli -- witness build register \
    --vector test-vectors/register/valid/register-valid-001.json >/dev/null
cargo run -q -p crucible-cli -- benchmark register --iterations 1 >/dev/null

echo "==> static checks"
bash scripts/check.sh

if command -v nargo >/dev/null 2>&1; then
    echo "==> nargo test (circuit workspace)"
    (cd circuits && nargo test)
    echo "==> circuit test vectors"
    bash scripts/generate-test-vectors.sh
    echo "==> cross-language vector runner (circuit tier)"
    cargo test -p crucible-tests --test vectors
    if command -v bb >/dev/null 2>&1; then
        echo "==> pinned artifacts: integrity check"
        cargo run -q -p crucible-cli -- artifacts check
        echo "==> pinned artifacts: fresh-compile determinism"
        cargo run -q -p crucible-cli -- artifacts generate --root /tmp/artifacts-fresh >/dev/null
        diff -r /tmp/artifacts-fresh artifacts/circuits
        echo "==> ultrahonk live proving + trait-seam + artifact-integrity"
        cargo test -p crucible-tests --test ultrahonk --test real_backend --test artifacts
    else
        echo "bb not on PATH; skipping live proving (see scripts/check-bb.sh)." >&2
    fi
else
    echo "nargo not on PATH; skipping circuit suites (see scripts/check-circuits.sh)." >&2
fi

echo "All tests passed."
