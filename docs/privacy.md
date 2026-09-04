# Privacy

The structural rules that keep private witness material out of logs,
errors, fixtures, Git history, and CI output — by construction rather than
by discipline.

## What must stay private

Private witness material: secret keys, amounts, blindings, commitment
openings — everything a circuit takes as a private parameter. The
repository draws one hard line:

> Values that must stay private are values **that cannot be printed,
> logged, serialized, or embedded in an error** — enforced in the type
> system, not in code review.

## Mechanisms

| Mechanism | Where |
| --- | --- |
| `SecretValue` implements no `Debug`, `Display`, or `Serialize` | `interfaces/src/circuit` |
| `ProofRequest` (which carries secrets) has no `Serialize`; only a redacted JSON view exists | `interfaces/src/proof_provider/request.rs` |
| `Debug` on requests/witnesses prints names and counts, never values | interfaces, `crucible-witness` |
| The witness **encoder** is the single place private values leave memory; writes `Prover.toml` at `0600` | `crates/witness/src/encoder.rs` |
| Toolchain stderr is never echoed into errors (compiler diagnostics can embed source snippets) | `crucible-noir`, `crucible-ultrahonk` |
| Prove/verify transcripts store redacted views only | `crates/prover-core/src/transcript.rs` |
| Witness files are gitignored except the explicitly synthetic `testdata/Prover.toml` fixtures | `.gitignore` |

## What is public by design

Not everything is secret. Proof **envelopes** are public — proof bytes,
public outputs, and provenance carry no private values, which is exactly
why they can be committed as fixtures ([docs/proof-format.md](proof-format.md)).
Public inputs (addresses, commitments, state roots) are visible in the
Confidential Token model ([docs/public-inputs.md](public-inputs.md)).

## Test coverage

The boundary is tested, not assumed:

- `SecretValue`/request `Debug` and redacted JSON never contain secret
  values (`tests/tests/security/witness_leakage.rs`, unit tests in
  `interfaces`).
- Prover errors, service transcripts, proof bytes, and response JSON never
  contain secret values.
- A "leak scanner" test verifies the harness itself can detect planted
  secrets, so the tests cannot silently go blind.
- Circuit-tier vectors only ever carry sample keys (`0x1234`), and the
  vectors crate routes them through the same `SecretValue` path as live
  material so the code path is identical without real secrecy.

## Threat framing

See [docs/security.md](security.md) (guarantees and mechanisms) and
[docs/threat-model.md](threat-model.md) (adversaries and leak channels).
The core privacy promise of Confidential Tokens — balances and amounts
hidden, senders/recipients visible — is mirrored structurally here: private
amounts cannot *become* public inputs, because the two sides are separate
bag types that the circuits never confuse ([docs/circuit-model.md](circuit-model.md)).
