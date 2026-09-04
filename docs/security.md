# Security

This repository handles the two things a proof system must never get wrong:
**private witness material** and **the integrity boundary between a proof
and the artifact that produced it**. This document states the guarantees,
the mechanisms, and how they are tested.

See [`threat-model.md`](./threat-model.md) for the adversarial view, and
[`SECURITY.md`](../SECURITY.md) for how to report a vulnerability.

## Guarantees

### G1 — Private witness values never leak

Private values (`SecretValue`, `PrivateWitnessBag`, `WitnessData`) are
structurally incapable of leaking by accident:

- `SecretValue` implements **no** `Debug`, `Display`, or `Serialize`. It
  cannot be formatted, logged, or JSON-encoded; the only escape hatch is the
  explicit, consuming `SecretValue::into_hex()` used by the encoder.
- `ProofRequest` carries secrets, so it has no `Serialize`. Its only JSON
  path is `redacted()`, which emits names and counts, never values.
- `Debug` views are redacted by construction.
- Toolchain (nargo) stderr is never echoed into errors, because compiler
  diagnostics can contain source snippets with witness values.
- The witness encoder writes `Prover.toml` with restrictive permissions and
  its output is never logged.

**Tested by**: `tests/security/witness_leakage.rs` and unit tests in
`interfaces`, `witness`, and `mock` asserting secrets never appear in debug
output, errors, transcripts, or JSON.

### G2 — Tampered proofs are rejected

A proof is bytes; anyone can flip bytes. Crucible detects this at two
levels:

- the mock backend keys a digest over its envelope payload, so any byte
  flip breaks verification (`InvalidProof`);
- real backends provide their own cryptographic soundness — the mock only
  exists to exercise the same rejection paths in CI.

**Tested by**: `tests/proofs/tampered.rs`, `tests/verification/corrupted_proof.rs`.

### G3 — Proofs are context-bound

A proof valid for state A, outputs X, key K, circuit C must fail when any of
those change:

- different public outputs → `PublicOutputMismatch`
- different state root → `StateReferenceMismatch` (stale-state / replay)
- different verification key → `WrongVerificationKey`
- different circuit or version → `CircuitMismatch` / `VersionMismatch`

**Tested by**: `tests/verification/*`, `tests/security/*`, and
`tests/invariants/*`.

### G4 — Artifacts are integrity-checked before use

`crucible-artifacts` refuses to load an artifact whose bytes do not match
its manifest: missing files, extra files (strict mode), and single-bit
flips all abort the load. Manifest paths are validated against path
traversal, and a manifest's own checksum can be pinned externally to detect
manifest tampering (file hashes alone cannot, since an attacker who can
replace files can replace the manifest).

The guarantee is *in the proving path*, not just in a library: the
UltraHonk provider proves only from the pinned artifact root
(`artifacts/circuits/<op>/`), strict-loading each artifact through this
loader before any witness is solved or `bb` runs. `artifacts check`
re-runs the same load from the CLI, and CI additionally diffs a fresh
compile against the committed artifacts so a circuit change that forgets
to re-pin fails the build instead of proving against stale bytecode.

**Tested by**: `tests/security/artifact_tampering.rs`, the artifact crate
unit suite, and the live `tests/tests/artifacts.rs` (tampered bytecode,
missing manifest/bytecode, and planted files against a copy of the pinned
artifact — all rejected before any proving work).

### G5 — Verification is not assumed equivalent across verifiers

Local verification and on-chain verification are different code paths.
`crucible-verifier` runs a proof through every verifier registered for its
backend and reports disagreement explicitly.

**Tested by**: `tests/integration/*` and the verifier crate unit suite.

## Privacy rules for contributors

1. Never add `Debug`, `Display`, or `Serialize` to a type that can hold
   private witness values.
2. Never include a witness value in an error message, log line, panic, or
   test fixture. Error messages carry names and identifiers only.
3. Never echo toolchain stderr verbatim into errors.
4. Never commit `Prover.toml`, generated proofs, or generated artifacts with
   real secrets (`.gitignore` covers generated paths; committed fixtures
   live under `test-vectors/` and contain no real secrecy).
5. When in doubt, run the leakage tests: they scan debug output, errors,
   transcripts, and JSON for known secret values from the fixtures.

## Testing philosophy

The mock backend makes security *tests* deterministic and fast, and because
its envelopes are self-describing it can say exactly why a proof was
rejected. That diagnostic power is a test double's feature, not a real
backend's — security tests that assert specific `VerificationFailure`
reasons encode the mock's behavior and are complemented by the real-backend
tests in `tests/tests/ultrahonk.rs` (tampered proofs, wrong verification
keys, and changed public inputs must all fail real `bb` verification).

The repository forbids `unsafe` code workspace-wide (`unsafe_code =
"forbid"`): witness and verification-key material is handled here, so the
code must stay in safe Rust.
