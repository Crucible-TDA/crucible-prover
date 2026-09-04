# Testnet

Design for an optional execution layer that runs the real Soroban verifier
contract against proofs — and why it is deliberately kept out of ordinary
tests and CI. **Status: design only; not implemented.**

## Purpose

Local verification proves a proof is cryptographically valid. Testnet
execution proves the *whole pipeline* works against reality: calldata
encoding accepted by a deployed verifier contract, transaction submission,
and on-chain acceptance/rejection. It is the natural extension of the
cross-verifier agreement idea ([docs/verification.md](verification.md),
[docs/soroban-verification.md](soroban-verification.md)) into a live
environment.

## Separation from ordinary tests

Like the toolchain gates, testnet execution should be an explicit, opted-in
surface, never a default CI dependency:

- Local dev and CI run mock and real-local suites only (`scripts/check.sh`,
  `scripts/test-all.sh`, the workspace test suite).
- A testnet run requires (a) the Soroban adapter, (b) a funded/configured
  network endpoint, and (c) a deployed verifier contract — all external
  state that ordinary tests must not depend on.

The existing pattern to follow is `scripts/check-bb.sh` /
`scripts/check-circuits.sh`: a gate script that exits non-zero in CI when
the prerequisite is missing but prints a clear skip locally. A
`scripts/check-testnet.sh` would gate testnet runs the same way.

## Developer-preview caveat

Stellar's Confidential Tokens are a developer preview: the contracts and
verifier remain under audit and are **not intended for production use**.
Testnet results are therefore integration evidence for the architecture,
not a production assurance — the same caveat that applies to the whole
proving stack until the circuit scheme is aligned with the Confidential
Token specification ([docs/deployment.md](deployment.md),
[docs/circuit-model.md](circuit-model.md)).

## Workstream order

1. Soroban verifier integration (adapter + agreement tests) —
   [docs/soroban-verification.md](soroban-verification.md).
2. Simulator adapter, so testnet scenarios run real state transitions —
   [docs/simulator-integration.md](simulator-integration.md).
3. Testnet configuration + submission + verification on top of both.
