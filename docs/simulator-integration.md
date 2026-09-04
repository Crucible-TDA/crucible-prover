# Simulator integration

How `crucible-prover` connects to `crucible-simulator` — the boundary, the
seams that exist today, and the adapter that will translate between the two
repositories.

## The boundary

The polyrepo split is strict:

> `crucible-simulator` owns the state and execution model.
> `crucible-prover` owns proving.

This repository therefore contains **no state engine**: no balances, no
ledger, no execution. It answers one question — *can I construct and verify
the proof for this state transition?* — given the state a simulator
provides. Duplicating the simulator's state engine here is an explicit
anti-goal ([docs/architecture.md](architecture.md)).

## Seams that exist today

Everything the simulator will touch is already defined, and crucially it is
defined **in the interface crate, not in any backend**:

- [`ProofProvider`](../interfaces/src/proof_provider/provider.rs) — the
  contract every backend (mock, ultrahonk) implements. The simulator never
  depends on a concrete prover.
- [`Prover`](../interfaces/src/prover/prover.rs) — the client-facing facade
  (`ProverService`) that validates, dispatches, and round-trip-verifies.
- `ProofRequest` / `ProofResponse` — the wire shapes, versioned and
  privacy-correct: requests carry private witness bags (no `Serialize`, only
  a redacted view for simulator logs), responses are public and traceable
  ([docs/proving-model.md](proving-model.md)).
- `StateReference` — the `(root, sequence)` pair that lets the simulator
  name *which* state a proof applies to, and lets the verifier reject
  stale/replayed proofs structurally
  ([docs/verification.md](verification.md)).

## The adapter (design, not yet implemented)

An `adapters/simulator` crate will translate simulator operations into
requests without importing the simulator's state engine:

```text
SimulatorOperation ──► ProofRequest ──► ProverService ──► ProofResponse
     (state + intent)     (witness bags + state reference)     (envelope)
```

Its job is mapping — simulator account/commitment/operation types onto the
circuit's named public/private inputs ([docs/public-inputs.md](public-inputs.md))
and routing through the `Prover` trait with the backend chosen per test
depth (mock for fast scenario runs, ultrahonk for real proofs). The mock's
TEST ONLY label and the scenario suites' freedom to pick per-depth backends
are what make the simulator tests fast today and honest when they need to
be ([docs/proving-model.md](proving-model.md)).

It is not implemented here because it needs the simulator's concrete types
(its own repository). Until it lands, the interface crate, the vector
catalog, and the examples demonstrate the exact contract the adapter will
implement.
