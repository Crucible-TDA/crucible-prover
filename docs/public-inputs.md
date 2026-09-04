# Public inputs

What a proof binds to, how the public side is named, and what happens when
it drifts from the circuits.

## Two roles, one value model

Public **inputs** (what the prover commits to when building a proof) and
public **outputs** (what the circuit reports when it runs) are both ordered
`FieldValue` bags over the same canonical-hex value model:

| Role | Type | Direction |
| --- | --- | --- |
| Inputs a request commits to | `PublicInputBag` | request → witness → circuit |
| Outputs the circuit reports | `OutputBag` | circuit → public outputs in the proof |

Names never cross the private boundary: a value is either a public `Field`
parameter (visible) or a private witness (never logged), enforced by the
witness model ([docs/witness-model.md](witness-model.md)).

## The per-operation surface

The exact public surface of each circuit is a shared spec, not a Rust
convention: `interfaces::circuit::expectations` pins, per operation, the
**ordered public parameter names** and the **public word count** the
compiled circuit must report. The real UltraHonk provider verifies the
word count against the pinned surface after every proof and refuses to name
words when the counts disagree — a circuit source that drifts from the
expectations spec fails loudly instead of mislabeling outputs.

The current operation surfaces (see `docs/circuit-model.md` for full
signatures):

- `register` — public: `account_address`; returns nothing.
- `deposit` — public: `token_address`, `account_address`, `old_commitment`;
  returns `(new_commitment, nullifier)`.
- `merge` — public: token + owner + two old commitments + `root_hi`,
  `root_lo`; returns `(new_commitment, nullifier_a, nullifier_b)`.
- `transfer` — public: token, sender/recipient addresses, old sender
  commitment, `root_hi`, `root_lo`; returns `(recipient_commitment,
  change_commitment, nullifier)`.
- `withdraw` — public: token, account address, commitment, `root_hi`,
  `root_lo`; returns `(change_commitment, nullifier)`.

Addresses are public — that is the Confidential Token privacy shape, where
who moves value is visible but how much is not.

## State binding

State-bound operations (merge, transfer, withdraw) carry a
`StateReference` whose 256-bit root is committed to as **two 128-bit field
halves** (`root_hi`, `root_lo`) — a full root does not fit one BN254 field.
The halves are folded into the circuit's nullifier, so a proof cut against
root A cannot be replayed against root B ([docs/ultrahonk.md](ultrahonk.md),
[docs/security.md](security.md)). The split convention is shared by the
fixtures, the witness path, and the verifier's structural checks
(`StateReference::root_halves`).

## Binding and mismatch

A proof's public outputs are part of its envelope, and verification checks
that the submitted context matches the proof (see
[docs/verification.md](verification.md)). Changing any bound public output
after the proof exists fails verification — pinned by
`tests/tests/invariants/public_inputs_bound.rs` and the security suite's
wrong-context tests.
