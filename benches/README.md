# benches

In-process benchmarks for the toolchain-free parts of the proving
pipeline. Run them with:

```bash
cargo bench -p crucible-benches
```

## Targets

| Target | Measures |
| --- | --- |
| `round_trip` | mock prove → verify → envelope JSON per operation (Crucible's orchestration overhead, no cryptography) |
| `witness_build` | request → `WitnessData` → `Prover.toml` encoding |
| `serialization` | envelope `to_json` / `from_json` round trip |

Timings are best-of-N single-shot samples in nanoseconds per operation,
printed per vector. They are deliberately dependency-free (`harness =
false`, `std::time` only) so `cargo bench` always runs anywhere.

## What these do NOT measure

- **Real UltraHonk proving / verification** — that needs `bb` and is
  measured live by `crucible-prover benchmark --backend ultrahonk` and
  the CI live suites (`cargo test -p crucible-tests --test ultrahonk
  --test real_backend`). Real cryptographic costs must never be inferred
  from mock numbers.
- **Circuit constraint counts** — those come from `nargo info` per
  gadget (`circuits/gadgets/*`); see `docs/circuit-model.md`.
