# Circuit model

The Noir circuits that give the proving layer its statements: what each
operation proves, where the public/witness boundary sits, the invariants
they enforce, and their measured cost.

## The state model the circuits operate on

Confidential value lives in **commitments** on the ledger:

```
commitment = Pedersen(COMMITMENT_DOMAIN, amount, blinding)
```

The ledger stores commitments; amounts and blindings never appear on it.
An account is a public **address** = key-hash of a secret key; the secret
is the only thing that can spend the account's commitments.

Operations consume old commitments (emitting a **nullifier** so they cannot
be spent twice) and produce new ones. Every nullifier binds the commitment,
the owner secret, and — for value-moving operations — the **token**, so a
proof cut for one token cannot be replayed against another.

```
public:  token, addresses, commitments being consumed/produced
private: account secret, amounts, blindings
```

## Operation circuits

### register

Proves ownership of a fresh account: `account_address = key_hash(account_sk)`.

- **public:** `account_address`
- **private:** `account_sk`
- **emits:** nothing (no state consumed)

### deposit

Adds confidential value to an owned commitment.

- **public:** `token_address`, `account_address`, `old_commitment`
- **private:** `account_sk`, `old_amount`, `old_blinding`, `amount`, `blinding`
- **proves:** ownership; `old_commitment` opens to `(old_amount,
  old_blinding)`; `amount` in range; emits a token-bound nullifier.
- **returns:** new commitment over `old_amount + amount`

### merge

Consolidates two owned commitments into one.

- **public:** `token_address`, `account_address`, `commitment_a`,
  `commitment_b`, `root_hi`, `root_lo`
- **private:** `account_sk`, both openings, `blinding`
- **proves:** ownership; both commitments open to their witness; emits two
  token- and state-bound nullifiers (root halves folded in).
- **returns:** merged commitment over `amount_a + amount_b`

### transfer

Moves confidential value from the sender to the recipient.

- **public:** `token_address`, `sender_address`, `recipient_address`,
  `old_sender_commitment`, `root_hi`, `root_lo`
- **private:** `sender_sk`, `amount`, sender opening, `recipient_blinding`,
  `change_blinding`
- **proves:** sender ownership; the sender's commitment opens to
  `old_amount`; `amount <= old_amount` (no overdraw); `amount` in range.
- **returns:** recipient commitment over `amount`, change commitment over
  `old_amount - amount`, token- and state-bound nullifier.

Recipient *binding* — forcing the produced recipient commitment to actually
belong to the public recipient address — is a protocol-level statement that
the scaffold does not yet enforce inside the circuit (see scope honesty
below and `docs/security.md`).

### withdraw

Redeems confidential value out of the domain.

- **public:** `token_address`, `account_address`, `commitment`,
  `root_hi`, `root_lo`
- **private:** `account_sk`, `amount`, `old_amount`, `old_blinding`,
  `change_blinding`
- **proves:** ownership; the commitment opens to `old_amount`;
  `amount <= old_amount`; emits a token- and state-bound nullifier.
- **returns:** change commitment over `old_amount - amount`, nullifier.

## Invariants enforced in-circuit

1. **Ownership** — only the account secret passes the address assertion.
2. **No overdraw** — `spend <= balance` (bounded u64 comparison).
3. **Range** — every amount is constrained to 63 bits before arithmetic, so
   field wraps cannot mint value.
4. **Opening consistency** — a consumed commitment must open to the witness
   provided; the ledger-facing commitment is bound to the transition.
5. **Conservation** — value in the produced commitments equals value out of
   the consumed ones (commitments are computed, not chosen).
6. **Double-spend protection** — consuming a commitment emits a nullifier.

State-bound operations (merge, transfer, withdraw) scope their nullifiers
to token **and** state: the two halves of the ledger state root are public
inputs folded into every emitted nullifier, so a proof cut for root A is
cryptographically rejected against root B (see `docs/ultrahonk.md`). Tree
*membership* of the consumed commitments in that root is a separate
statement (it needs an inclusion proof) and lands with the membership
workstream; register and deposit remain state-unbound by design.

## Scope honesty

These circuits implement the *shape* of the Confidential Token semantics so
the proving architecture can be exercised end to end, but the scaffold must
be aligned with the real specification before production use:

- commitment layout and hash (Pedersen via the Noir stdlib here),
- key derivation and address format,
- nullifier construction (including the exact token/domain folding),
- the transfer recipient-binding statement,
- whether merge/withdraw exist as separate operations in the target design.

Each deviation is flagged at its definition site in `circuits/lib/`. The
test vectors in `circuits/*/testdata/` and the JSON fixtures are derived
from these scaffold semantics and will need regeneration on alignment.

## Measured cost (nargo 1.0.0-beta.26)

`nargo info` per operation circuit (`main` function, ACIR / Brillig
opcodes) and per measurement gadget:

| Package | ACIR | Brillig | Notes |
| --- | --- | --- | --- |
| `register` | 10 | 44 | ownership only |
| `deposit` | 96 | 61 | consume + produce + range |
| `merge` | 124 | 61 | two consumes + produce |
| `transfer` | 118 | 61 | consume + two produces + overdraw |
| `withdraw` | 100 | 61 | consume + produce + overdraw |
| `gadget_commitment` | 18 | 44 | one Pedersen commitment |
| `gadget_hash` | 18 | 44 | one two-field hash |
| `gadget_range` | 16 | 17 | 63-bit amount range check |
| `gadget_ownership` | 11 | 44 | address ownership assertion |
| `gadget_state` | 62 | 44 | full consume-and-produce transition |

The gadget rows are the reference points for reasoning about circuit cost:
the value-moving operations sit just above `gadget_state` plus one range
check per additional amount, while register is essentially `gadget_ownership`.
Regenerate with `nargo info --workspace`; treat these as drift canaries,
not fixed promises, until a backend pins real constraint counts.
