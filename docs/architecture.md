# Architecture

`crucible-prover` is the **proof engine** of the Crucible polyrepo. It owns
zero-knowledge proof generation, verification, proof artifacts, circuit
interfaces, and prover backends. The other two polyrepos own their own
domains and the boundaries between them are strict:

> `crucible-simulator` owns the state and execution model.
> `crucible-prover` owns proving.
> `crucible-scenarios` owns scenario orchestration and adversarial testing.

This document describes the internal architecture of `crucible-prover` and
the rules that keep it from becoming a dumping ground.

## Repo layout at a glance

```text
interfaces/        stable contracts shared by every crate and sibling repo
crates/
  proof-types/     versioned wire format (ProofEnvelope)
  witness/         private witness assembly, encoding, redaction
  artifacts/       artifact manifests, checksums, integrity-gated loading
  mock/            TEST-ONLY deterministic prover/verifier
  prover-core/     provider registry, dispatch, orchestration
  verifier/        verification dispatch + cross-verifier agreement
  noir/            nargo CLI adapter (compile/execute/info)
  ultrahonk/       UltraHonk backend identity + calldata encoding
schemas/           JSON Schema contracts for the wire formats
scripts/           check.sh / test-all.sh and helpers
docs/              this and the other architecture docs
circuits/          Noir circuit workspace (arrives with the circuits batch)
tests/             cross-crate security/invariant suite (this batch)
```

## The canonical flow

```text
Simulator State
      │
      ▼
Proof Request            interfaces::ProofRequest
      │
      ▼
Witness Construction     crucible-witness (private ↔ public split)
      │
      ▼
Circuit                  circuits/ (Noir, later batch)
      │
      ▼
ACIR                     nargo compile (crucible-noir)
      │
      ▼
Prover Backend           ProofProvider impl (mock today, UltraHonk later)
      │
      ▼
ZK Proof                 ProofResponse / ProofEnvelope
      │
      ▼
Public Inputs            bound to the response
      │
      ▼
Verification             crucible-verifier (local + on-chain agreement)
```

## Layer rules

### 1. Clients depend on interfaces, never on backends

`crucible-simulator` and `crucible-scenarios` depend on
`crucible-interfaces` only:

```text
simulator / scenarios
      │  (ProofProvider / Prover traits)
      ▼
   interfaces
      ▲
      │
 mock · noir · ultrahonk
```

This is what lets the simulator tests stay fast (mock proofs), prover
implementations evolve independently, and scenarios pick mock or real proofs
per test depth.

### 2. The repository is not UltraHonk-specific

UltraHonk is the backend of the current Stellar Confidential Token
implementation, but Crucible is the *proof engine*, not an UltraHonk shim.
The `ultrahonk` crate contains only backend-specific knowledge — format
tags, version compatibility, verification-key-id policy, calldata
encoding — and plugs in through the same `ProofProvider`/`Verifier` traits
the mock uses. RISC Zero/Groth16 or any future Stellar verifier
architecture plugs in the same way.

### 3. The circuit/toolchain boundary is explicit

Noir is not "just another Rust crate". `nargo` is its own toolchain: it
compiles `circuits/` into ACIR artifacts independently of Cargo, and
`crucible-noir` is the only crate allowed to execute it. Proof generation
and verification are not nargo's job in current toolchains — they moved to
the Barretenberg backend — so the `ultrahonk` crate will consume this
crate's artifacts and witnesses when the backend adapter lands.

### 4. Integrity before trust

Every compiled artifact is loaded through `crucible-artifacts`, which
refuses to hand out bytes that do not match the artifact's manifest
byte-for-byte. Nothing is ever loaded partially, and no proof is ever
accepted from an artifact that failed its checksum.

### 5. Verification round-trip is mandatory by default

`ProverService::prove_and_verify` refuses to return a proof that fails
against its own response. `crucible-verifier` goes further: it can run the
same proof through every verifier registered for a backend (local,
on-chain) and report disagreement instead of assuming equivalence.

### 6. Privacy is structural

Private witness values cannot be formatted, logged, serialized, or embedded
in errors by accident: `SecretValue` implements no `Debug`, `Display`, or
`Serialize`. `ProofRequest` carries secrets and therefore has no
`Serialize`; its only JSON form is the redacted view. Toolchain stderr is
never echoed into errors because compiler diagnostics can contain source
snippets with witness values.

## What does NOT belong in this repo

- a wallet, token contract, blockchain explorer, or transaction simulator
- a compliance, audit, or policy engine
- a general-purpose ZK framework or production key-management system
- the entire OpenZeppelin Confidential Token implementation
- large scenario catalogs, stress/adversarial/concurrency scenarios
  (those are `crucible-scenarios`' job)

`crucible-prover` answers one question: *can I construct and verify the
proof for this state transition?* `crucible-scenarios` answers: *what
happens when I execute hundreds of valid, invalid, adversarial, and
pathological transitions?*

## Repository status

Batch 1 (this state of `main`) lays the proving foundation: interfaces,
wire types, witness and artifact management, mock backend, prover-core
orchestration, the verification service, the nargo adapter, UltraHonk
calldata knowledge, JSON schemas, and the security/invariant suite. The
Noir circuits, real UltraHonk proving via Barretenberg, Soroban
verification, and the CLI arrive in later batches on top of these seams.
