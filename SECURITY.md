# Security Policy

`crucible-prover` is the proof engine of the Crucible Confidential Token
test suite. It handles material that must never be exposed: private witness
values, secret randomness, and secret openings.

## Reporting a vulnerability

Do **not** open a public issue for a security vulnerability. Report it
privately by opening a [security advisory][gh-advisory] on GitHub, or by
emailing the maintainers (see `CONTRIBUTING.md`).

Please include:

- the affected crate/version and the exact operation that triggers the bug
- a minimal reproduction (test vector, proof request, or code snippet)
- your assessment of impact, especially whether any private witness value,
  secret, or verification key material can be leaked or forged

You should receive an acknowledgement within 5 business days.

## What this project considers in scope

- Leakage of private witness, secret randomness, or secret openings through
  logs, errors, serialization, CI output, or public fixtures
- Acceptance of tampered, stale, replayed, or misattributed proofs
- Verification-key or artifact integrity failures (checksum bypass)
- Circuit/public-input binding violations
- Unauthorized proof generation or verification bypasses

## Out of scope

- The underlying cryptographic soundness of the Noir circuits or the
  UltraHonk proving system itself. Defects in upstream provers
  (Noir/Barretenberg) must be reported to their respective projects.

## Security stance

This repository is part of the Crucible test suite for a protocol that
Stellar labels a **developer preview** and which remains under audit. The
`mock` prover shipped in this workspace is explicitly **not
cryptographically secure** and must never be used outside tests.

[gh-advisory]: https://github.com/Crucible-TDA/crucible-prover/security/advisories/new
