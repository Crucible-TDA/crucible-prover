# crucible-prover

The **proof engine** of [Crucible](https://github.com/Crucible-TDA), a
three-part test suite for the Stellar Confidential Token architecture:

| Repository | Responsibility |
| --- | --- |
| `crucible-simulator` | Simulate: deterministic state and execution model |
| **`crucible-prover`** | **Prove: zero-knowledge proof generation, circuits, verification, proof artifacts** |
| `crucible-scenarios` | Stress-test: scenario orchestration and adversarial testing |

The boundary is strict:

> `crucible-simulator` owns the state and execution model.
> `crucible-prover` owns proving.
> `crucible-scenarios` owns scenario orchestration.

## What this repository does

`crucible-prover` owns the complete proving lifecycle:

```text
Simulator State ─▶ Proof Request ─▶ Witness ─▶ Circuit ─▶ ACIR
      ─▶ Prover Backend ─▶ ZK Proof ─▶ Public Inputs ─▶ Verification
      ─▶ Soroban-Compatible Proof
```

It is **not** a wallet, a token contract, a blockchain explorer, a
transaction simulator, a compliance or audit engine, a general-purpose ZK
framework, a secrets vault, or a Soroban SDK. It is the proof engine and
proving infrastructure for Crucible.

## Layout

```text
interfaces/    Stable contracts: ProofProvider/Verifier traits, requests, circuit ids
crates/        prover-core, proof-types, witness, artifacts, noir, ultrahonk,
               verifier, mock
adapters/      Bridges to the simulator, Soroban, and a testnet
circuits/      The Noir workspace (shared lib, register/deposit/merge/transfer/
               withdraw circuits, measurement gadgets)
artifacts/     Compiled circuits, verification keys, manifests
test-vectors/  Valid/invalid vectors per operation (cross-language fixtures)
schemas/       JSON schemas for proofs, requests, witnesses, artifacts
proofs/        Proof fixtures and serialization compatibility material
tests/         Unit, circuit, proof, verification, security, invariant suites
cli/           Orchestration CLI (no proving logic)
docs/          Architecture and design documents
scripts/       Toolchain setup, circuit checks, vector generation
```

## Status

Under active construction. The canonical end-to-end flow is:

```text
simulator state ─▶ witness builder ─▶ Noir circuit ─▶ ACIR
      ─▶ UltraHonk prover ─▶ ZK proof ─▶ local verifier / Soroban verifier
```

Backends plug into the `ProofProvider` interface so the simulator and the
scenario suites never couple to UltraHonk — or to the `mock` prover used in
CI, which is **TEST ONLY and NOT cryptographically secure**.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Noir (circuits) is a separate toolchain; see `scripts/setup-noir.sh` and
`docs/noir.md`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE) or
[MIT license](LICENSE-MIT) at your option.
