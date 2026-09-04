# Verification

How proofs are checked, why local and on-chain verification must not be
assumed equivalent, and the structural checks that run before (and
alongside) cryptography.

## The Verifier trait

A [`Verifier`](../interfaces/src/verifier/verifier.rs) takes a
`VerificationRequest` — built losslessly from a `ProofResponse`/envelope —
and returns a structured `VerificationOutcome` (verified, or a specific
`VerificationFailure` reason: invalid proof, wrong verification key,
public-output mismatch, state-reference mismatch, …). Implementations:

- `MockVerifier` — deterministic envelope checker for the test-only mock
  backend (no cryptography).
- `UltraHonkVerifier` — runs the real `bb verify` over the submitted
  context, resolving the verification key from the `VkStore` by id.

## VerificationService: cross-verifier agreement

Local verification and on-chain verification are **not assumed
equivalent** without testing. `crucible-verifier`'s
`VerificationService` registers several verifiers per backend (e.g.
`local` and, later, `soroban`, both serving `ultrahonk`) and runs a proof
through all of them, returning a `VerificationReport` that states whether
they **agreed**. Disagreement is surfaced, never hidden
(`tests/tests/verification.rs`, `tests/src/` stack). Dispatch is keyed by
backend identity, so a mock proof can never accidentally reach an
UltraHonk verifier.

## Mandatory round trip

`ProverService::prove_and_verify` refuses to return a proof that fails
verification against its own response — a proof is not handed to a caller
until the local check passes. The CLI's `prove` and the examples inherit
this rule.

## Structural checks before crypto

Some failures are detected without running the backend, because they are
binding violations, not cryptography failures:

- **State binding** — a proof cut against state root A, submitted with a
  state reference of root B (or stripped entirely), is rejected
  (`StateReferenceMismatch`) by the verifier layer before/independent of
  backend work; the cryptographic nullifier binding is additionally
  exercised in the live suite.
- **Context binding** — wrong circuit, version, backend, or altered public
  outputs each produce distinct, structured rejections
  (`tests/tests/security/wrong_context.rs`).

## Failure taxonomy

The integration suites pin a mapping from tamper/attack to failure reason:
byte flips, truncation, splicing, and appended bytes → `InvalidProof`;
verification-key substitution → wrong-key rejection; public-output edits →
context mismatch; state-root substitution → state mismatch
(`tests/tests/security/`). Reasons are machine-readable so callers never
parse prose.

## Related

- [docs/proof-lifecycle.md](proof-lifecycle.md) — every stage a proof
  passes through and its failure modes.
- [docs/proof-format.md](proof-format.md) — the envelope that feeds
  verification.
- [docs/security.md](security.md) and [docs/threat-model.md](threat-model.md)
  — the guarantees and adversaries these checks implement.
