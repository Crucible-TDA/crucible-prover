# examples

Runnable demonstrations of the proving pipeline. They use the **mock
backend** — TEST ONLY, not cryptographically secure — so they run
anywhere with no toolchain and are meant to show the *shape* of the
flow, not real proofs. For real UltraHonk proving use the CLI:

```bash
cargo run -q -p crucible-cli -- prove transfer \
  --vector test-vectors/transfer/valid/transfer-valid-001.json \
  --backend ultrahonk
```

## `end-to-end`

Walks the full operation lifecycle against the committed vector catalog:
register → deposit → merge → transfer → withdraw, proving and verifying
each through the same service seams the CLI uses, and printing the
envelope summary (public-word count, proof size, bound state root).

```bash
cargo run -p crucible-examples --bin end-to-end
```

## `prove-verify`

Prove one operation and verify it locally:

```bash
cargo run -p crucible-examples --bin prove-verify -- transfer
cargo run -p crucible-examples --bin prove-verify -- register --vector path/to/vector.json
```

The exit code is zero only when the proof passes its own round-trip —
the same mandatory round-trip rule the service enforces.

These binaries are demonstrations, not tests: the regression net lives
in `tests/` and `benches/`. Prefer adding coverage there.
