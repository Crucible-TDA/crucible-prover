# Proving Model

This document describes how a proof moves through `crucible-prover`: what a
proof request is, how providers are selected, what a proof response commits
to, and why every field on the wire exists.

## Requests

A [`ProofRequest`] is the single input format across Crucible. It carries:

```text
request_id        traceability across simulator, prover, scenarios
operation         register | deposit | merge | transfer | withdraw
circuit           the circuit that proves this operation
circuit_version   which version of the circuit (proofs are version-bound)
artifact_version  which compiled artifact generation is expected
backend           which proving system must serve this request
witness           PRIVATE material — never serialized or logged
public_inputs     public context the proof must bind to
state_reference   the state root + sequence the proof applies to
```

### Validation is layered

1. **Structural** (`ProofRequest::validate`): every operation requires a
   private witness and public inputs; state-bound operations (merge,
   transfer, withdraw) require a state reference.
2. **Circuit-level** (`prover-core::witness`): the operation's required
   private names must be present (`transfer` needs `sender_sk`, `amount`,
   `blinding`, etc.).
3. **Provider-level**: the provider registered for the request's backend
   must declare support for the circuit at the requested version before any
   proving runs.

Nothing reaches a backend until all three layers pass.

## The ProofProvider seam

```text
ProofProvider (trait, in interfaces)
      │
      ├── MockProver        deterministic, TEST-ONLY
      ├── NoirProver        (arrives with circuits + bb)
      └── UltraHonkProver   (arrives with circuits + bb)
```

`ProverService` (in `prover-core`) is the client-facing facade. It holds a
registry of providers, preflights each request, dispatches to the provider
for the request's backend, and wraps the result in a versioned
`ProofEnvelope`.

## Responses and binding

A [`ProofResponse`] is fully traceable: it names the circuit, circuit
version, backend, verification key id, and artifact checksum the proof was
produced against. Proofs are **bound** to their public context in two ways:

- the proof bytes commit to the public outputs and state reference, and
- verification checks the *submitted* context against the proof's embedded
  context field by field.

If public inputs change after proof generation, verification fails — that is
what prevents stale-state proof reuse.

## The proof envelope

The `ProofEnvelope` is the storage/exchange form of a proof:

```json
{
  "version": 1,
  "operation": "transfer",
  "circuit": "transfer",
  "circuit_version": "0.1.0",
  "backend": "mock",
  "proof": { "format": "mock-envelope-v1", "bytes": "..." },
  "public_outputs": { "entries": [["new_commitment", "c0ffee"]] },
  "verification_key_id": "mock-vk/transfer/0.1.0",
  "artifact_checksum": "<sha256>",
  "state_reference": { "root": "<sha256>", "sequence": 1 },
  "metadata": { "request_id": "...", "produced_by": "crucible-prover/0.1.0" }
}
```

Envelope parsing rejects future versions rather than guessing, so old
tooling can never silently misinterpret newer proofs.

## The mock backend (TEST ONLY)

The mock performs no cryptography. Its proofs are self-describing envelopes
that bind every public-context field, so the full provider/verifier
contract — validation, assembly, binding, rejection of tampered or
misattributed proofs — runs in CI without paying proving costs. Key
properties:

- **Deterministic**: the same request always yields the same proof bytes.
- **Tamper-evident**: flipping any byte breaks the keyed digest.
- **Diagnostic**: because the envelope is in the clear, verification can say
  *why* a proof was rejected (wrong key vs. stale state vs. tampered bytes).

The mock is loudly labelled NOT CRYPTOGRAPHICALLY SECURE and must never be
used where soundness is the point.

## Real backends

Compilation of Noir circuits and witness solving are nargo's job
(`crucible-noir`); proof generation and verification are the Barretenberg
backend's job, arriving with the circuits batch. The compatibility matrix in
`crates/ultrahonk/src/backend.rs` pins which `(nargo, bb)` pairs each
circuit version was validated against — an unvalidated pairing is explicit,
never assumed.
