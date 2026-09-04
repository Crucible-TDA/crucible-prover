# Performance

Where the costs are, how each is measured, and why the mock backend must
never be used to reason about real proving speed.

## Where the cost lives

The proving pipeline has four very different cost centers:

| Stage | Tool | Measured by |
| --- | --- | --- |
| Constraint count / circuit size | nargo (compile-time) | `nargo info` per package |
| Witness solving | nargo execute | live runs, CI |
| UltraHonk proving | bb prove | `benchmark --backend ultrahonk`, live suites |
| UltraHonk verification | bb verify | same |

Crucible's own orchestration (dispatch, envelope assembly, validation,
serialization) is a fourth, comparatively tiny cost — but it is the only
one the mock backend measures, because the mock does no cryptography.

## Measurement surfaces

1. **`nargo info` (constraint costs)** — every operation circuit and every
   measurement gadget under `circuits/gadgets/` reports its own ACIR/Brillig
   opcode counts. The gadget-per-primitive layout exists precisely so
   primitive-level cost can be tracked independently of whole-operation
   cost ([docs/circuit-model.md](circuit-model.md)).
2. **`benches/` (in-process, toolchain-free)** — mock round trip, witness
   build, and envelope serialization, best-of-N nanoseconds per operation
   ([benches/README.md](../benches/README.md)). These catch orchestration
   regressions anywhere, with zero cryptography.
3. **`crucible-prover benchmark <op>`** — per-phase timings (prove, verify,
   serialize) plus proof/envelope byte sizes over `--iterations`. With
   `--backend mock` it prints an explicit TEST ONLY warning; with
   `--backend ultrahonk` it times real proving through `bb`.
4. **Live suites** — `cargo test -p crucible-tests --test ultrahonk
   --test real_backend` prove and verify real witnesses in CI; they are
   correctness gates first, but their wall time is a coarse regression
   signal.

## Honest reporting rules

- **Mock numbers never imply crypto cost.** The mock warning exists so a
  µs round trip is not mistaken for proving speed.
- **Proof size and verification cost** are backend properties: envelope
  JSON length and proof bytes are reported by the CLI and benches; on-chain
  verification cost is a Soroban/calldata concern
  ([docs/soroban-verification.md](soroban-verification.md)).
- **No stored baselines yet.** These surfaces produce comparable numbers;
  wiring them into a baseline/comparison workflow (e.g. a `performance`
  issue template already exists under `.github/ISSUE_TEMPLATE/`) is the
  next step when real proving volume justifies it.
