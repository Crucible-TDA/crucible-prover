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
artifacts/     Pinned compiled circuits + manifests (proving input), VK store
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

## Documentation

- [`docs/architecture.md`](docs/architecture.md) — layers, boundaries, and
  the dependency rules that keep the repos decoupled
- [`docs/proving-model.md`](docs/proving-model.md) — requests, providers,
  binding, and the mock backend
- [`docs/proof-lifecycle.md`](docs/proof-lifecycle.md) — the stages every
  proof passes through and their failure modes
- [`docs/security.md`](docs/security.md) — guarantees, mechanisms, and
  privacy rules for contributors
- [`docs/threat-model.md`](docs/threat-model.md) — adversaries, attacks,
  and the defenses (and tests) that resist them
- [`docs/circuit-model.md`](docs/circuit-model.md) — the Noir operation
  circuits, their public/witness boundaries, and measured costs
- [`docs/noir.md`](docs/noir.md) — Noir toolchain split and the circuit
  workspace conventions
- [`docs/ultrahonk.md`](docs/ultrahonk.md) — real UltraHonk proving with
  the Barretenberg backend: the validated `nargo` × `bb` pairing and live
  proof coverage
- [`docs/test-vectors.md`](docs/test-vectors.md) — the cross-language vector
  catalog and the mock/circuit runner tiers that judge it
- [`docs/artifacts.md`](docs/artifacts.md) — the pinned-artifact model:
  manifests, the provider gate, and the CI freshness check
- [`docs/cli.md`](docs/cli.md) — the `crucible-prover` CLI: circuits,
  artifacts, prove/verify, and the catalog gate

## Quick start (CLI)

```bash
cargo run -q -p crucible-cli -- circuits list
cargo run -q -p crucible-cli -- circuits compile
cargo run -q -p crucible-cli -- artifacts check
cargo run -q -p crucible-cli -- prove transfer \
  --vector test-vectors/transfer/valid/transfer-valid-001.json \
  --backend mock
cargo run -q -p crucible-cli -- verify transfer-valid-001.mock.proof.json
cargo run -q -p crucible-cli -- vectors run
```

See [`docs/cli.md`](docs/cli.md) for the full command surface.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE) or
[MIT license](LICENSE-MIT) at your option.
