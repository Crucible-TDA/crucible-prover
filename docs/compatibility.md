# Compatibility

Versioning policy across the things that must agree for a proof to be
produced, stored, and verified: the wire format, the circuit/artifact
identity, the toolchain pairing, and the fixtures that pin them.

## Versioned surfaces

| Surface | Version carrier | Policy |
| --- | --- | --- |
| Proof envelope | integer `version` (current 1) | parsing **rejects** versions newer than known; never guess |
| Artifact manifest | `manifest_version` (current 1) + `artifact_version` | loader rejects unknown manifest schemas; `artifact_version` bumps on recompile without circuit change |
| Circuit | `circuit_version` (0.1.0) | a circuit change bumps it; backends gate on it |
| Backend | `BackendId` string (`mock`, `ultrahonk`) | open string set so new architectures (RISC Zero/Groth16) need no interface change |
| Proof encoding | `ProofFormat` tag (`mock-envelope-v1`, `ultrahonk-v1`) | names the encoding; bytes stay backend-owned |
| Verification key | `VerificationKeyId` | derived from circuit + version + artifact checksum |
| Toolchain pairing | `BACKEND_COMPAT` matrix | `(nargo, bb, circuit_version)` rows validated in CI |

## Rules that keep surfaces compatible

1. **Old tooling must fail loudly, not misread.** Envelope parsing rejects
   `version > 1`; manifest parsing rejects unknown schema versions; backend
   version gates refuse unvalidated nargo/bb pairings
   ([docs/ultrahonk.md](ultrahonk.md)).
2. **Identity travels with the proof.** A `ProofEnvelope` names circuit,
   versions, backend, vk id, artifact checksum, and state reference, so a
   consumer never guesses what produced a proof
   ([docs/proof-format.md](proof-format.md)).
3. **Version bumps are multi-surface.** Changing a circuit, the nargo pin,
   or the bb pin requires updating: the circuit source + expectations spec,
   the relevant `TESTED_*` constant and `BACKEND_COMPAT`, the installer
   scripts/CI pins, and re-pinning artifacts + regenerating fixtures — the
   fresh-compile determinism gate fails until they all agree
   ([docs/reproducibility.md](reproducibility.md)).
4. **Calibration lands as a new version, not a mutation.** The calldata
   encoder carries a format-version byte so on-chain layout calibration can
   ship without breaking stored fixtures ([docs/soroban-verification.md](soroban-verification.md)).

## Cross-version material

`proofs/fixtures/` (envelope v1 JSON, judged by `tests/tests/proof_fixtures.rs`)
and `test-vectors/` (judged by the two-tier runner) are the committed
regression nets that catch accidental format drift. Compatibility tests for
new envelope versions belong next to those nets: commit a v1 fixture, add
the v2 parser, prove the v1 fixture still parses and the v2 gate rejects
old tooling.
