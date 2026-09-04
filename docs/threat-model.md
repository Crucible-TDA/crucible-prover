# Threat Model

An adversarial description of what `crucible-prover` must resist. Each entry
names the adversary, the attack, and the defense that already exists or is
planned. The security test suite (`tests/security/`) maps one-to-one onto
the rows below.

## Assets

| Asset | Sensitivity | Where it lives |
| --- | --- | --- |
| Private witness values | HIGH — the privacy boundary itself | in-memory only, `Prover.toml` during proving |
| Verification keys | HIGH — forge proofs if leaked | artifact store, verified by checksum |
| Proving keys / secret backend material | CRITICAL — forge proofs if leaked | backend infrastructure, never in this repo |
| Compiled circuit artifacts | MEDIUM — must be authentic, not secret | `artifacts/`, integrity-checked |
| Proofs | LOW per-proof, but must be authentic & context-bound | envelopes, stored/exchanged |
| Public inputs / state roots | PUBLIC | everywhere, by design |

## Adversaries

### A1. The curious bystander (privacy)
**Goal**: learn a private balance, amount, or opening from observable data.

**Attack surface**: logs, error messages, debug output, CI artifacts,
transcripts, panics, serialized requests, git history.

**Defenses**: structural non-leakability (no `Debug`/`Display`/`Serialize`
on secrets); redacted `ProofRequest` JSON; redacted transcripts; nargo
stderr never echoed; `.gitignore` excludes generated witness/proof files;
leakage tests scan everything for fixture secret values.

**Status**: enforced today. Test: `tests/security/witness_leakage.rs`.

### A2. The forger (proof soundness)
**Goal**: produce a proof that verifies for a state transition that did not
happen, or for a context it does not commit to.

**Attack**: forge bytes, reuse a proof against different state/outputs/key.

**Defenses**: real backends provide cryptographic soundness (mock does not,
by design — it is TEST ONLY); context binding is checked field-by-field at
verification; `prove_and_verify` round-trips every proof it returns.

**Status**: enforced today — cryptographic soundness via the Barretenberg
adapter (`UltraHonkProvider`/`UltraHonkVerifier`), with the mock exercising
the same rejection paths in CI. Tests: `tests/tests/real_backend.rs`,
`tests/tests/ultrahonk.rs`, `tests/security/proof_malleability.rs`.

### A3. The replayer / stale-state submitter
**Goal**: submit a previously valid proof after the state it applies to has
moved on.

**Attack**: capture a valid proof for state root A; submit it when the
account state is at root B.

**Defense**: state-bound operations (merge, transfer, withdraw) require a
`StateReference`; verification compares the submitted reference against the
proof's binding and rejects with `StateReferenceMismatch` / `MissingStateBinding`.

**Status**: enforced today. Tests: `tests/security/replay.rs`,
`tests/security/stale_state.rs`, `tests/proofs/replay.rs`.

### A4. The artifact swapper (integrity)
**Goal**: make the system prove or verify with a modified circuit, or leak a
verification key by convincing the loader to read outside the artifact root.

**Attack**: replace artifact bytes and/or its manifest; plant an extra file;
craft a manifest whose paths escape the artifact directory.

**Defense**: `crucible-artifacts` verifies every file against its manifest
before any content is returned, rejects undeclared files in strict mode,
and validates paths against traversal; manifests carry their own checksum
pinnable outside the artifact directory. The UltraHonk provider *proves
only from a pinned artifact root* (`artifacts/circuits/<op>/`): it
strict-loads the artifact through that loader before a single byte is
touched, so swapped or tampered bytecode fails with
`ArtifactIntegrity`/`ArtifactUnavailable` before any witness is solved or
`bb` runs. A `circuits`-source change that forgets to re-pin its artifact
fails CI (fresh compile must match the committed bytes byte-for-byte).

**Status**: enforced today — pinned artifacts committed for all five ops,
verified by `crucible-prover artifacts check`. Tests:
`tests/security/artifact_tampering.rs` and the live `tests/tests/artifacts.rs`
(byte-flip, missing manifest/bytecode, and planted-file attacks against a
copy of the pinned artifact).

### A5. The misattribute (circuit/key confusion)
**Goal**: pass off a proof for circuit A (or version 1.0) as a proof for
circuit B (or version 2.0), or under a different verification key.

**Defense**: proofs bind circuit, circuit version, backend, verification key
id, and artifact checksum; verification checks each field and rejects with
the specific reason.

**Status**: enforced today. Tests: `tests/security/wrong_context.rs`,
`tests/security/key_mismatch.rs`, `tests/verification/wrong_key.rs`,
`tests/verification/wrong_public_inputs.rs`.

### A6. The toolchain saboteur
**Goal**: make compilation or proving fail in a way that leaks witness
values through diagnostics, or make the system silently use an incompatible
toolchain.

**Defense**: `crucible-noir` never echoes nargo stderr into errors; the
toolchain adapter checks the nargo version against the supported major
before running; `crucible-ultrahonk` gates the `bb` major version and CI
installs the exact `(nargo, bb)` pair pinned in the compatibility matrix
(`noirup -v 1.0.0-beta.26` + `bbup -v 6.0.0-nightly.20260903`) — the same
versions that generated the committed pinned artifacts.

**Status**: enforced — nargo and bb toolchains are both gated, and the live
proving suite in `tests/tests/ultrahonk.rs` exercises the validated pairing
on every CI run.

### A7. The environment attacker (unsafe code)
**Goal**: exploit memory unsafety in witness/key handling.

**Defense**: `unsafe_code = "forbid"` workspace-wide. All handling is safe
Rust.

**Status**: enforced today (workspace lint).

## Out of scope (deliberately)

- Side-channel resistance of real proving hardware/software (backend
  concern, addressed with the Barretenberg adapter).
- Consensus/chain security (the ledger's concern, not the prover's).
- Production key management (a different system; this repo never holds
  proving keys).
