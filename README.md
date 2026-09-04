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
interfaces/    Stable contracts: ProofProvider/Prover/Verifier traits,
               requests/responses, circuit ids, expectations spec
crates/        prover-core, proof-types, witness, artifacts, noir,
               ultrahonk, verifier, mock, vectors
circuits/      The Noir workspace (shared lib, register/deposit/merge/transfer/
               withdraw circuits, measurement gadgets)
artifacts/     Pinned compiled circuits + manifests (the proving input),
               runtime verification-key store
test-vectors/  Cross-language vectors per operation (valid + reject categories)
schemas/       JSON schemas for proofs, requests, witnesses, artifacts
proofs/        Committed proof-envelope fixtures + serialization material
tests/         Cross-crate security/invariant/verification/live suites
benches/       In-process pipeline benchmarks (toolchain-free)
examples/      Runnable end-to-end demos (mock backend)
cli/           Orchestration CLI (no proving logic)
docs/          Architecture and design documents
scripts/       Toolchain setup, gates, and regeneration scripts
```

## Status

The full proving pipeline is implemented and green in CI: interfaces and
wire types, witness and artifact management, mock and UltraHonk backends
proving only from manifest-pinned artifacts, state-bound circuits,
prover-core orchestration, cross-verifier agreement, the vector catalog,
committed proof fixtures, benchmarks, examples, and the `crucible-prover`
CLI (whose binary is attached to tagged releases). The mock backend is
TEST ONLY and not cryptographically secure.

The canonical end-to-end flow is:

```text
simulator state ─▶ witness builder ─▶ Noir circuit ─▶ ACIR
      ─▶ UltraHonk prover ─▶ ZK proof ─▶ local verifier / Soroban verifier
```

Backends plug into the `ProofProvider` interface so the simulator and the
scenario suites never couple to UltraHonk — or to the mock prover used in
CI.

Open workstreams (designed, not yet built): Soroban on-chain
verification, Merkle membership for consumed commitments, the simulator
adapter, and the optional testnet layer — see `docs/soroban-verification.md`,
`docs/simulator-integration.md`, and `docs/testnet.md`.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Noir (circuits) is a separate toolchain; see `scripts/setup-noir.sh` and
`docs/noir.md`.

## Documentation

**Architecture & lifecycle** — [`docs/architecture.md`](docs/architecture.md)
(layers, boundaries, dependency rules), [`docs/proving-model.md`](docs/proving-model.md)
(requests, providers, binding, mock), [`docs/proof-lifecycle.md`](docs/proof-lifecycle.md)
(stages and failure modes), [`docs/witness-model.md`](docs/witness-model.md)
(private/public split, encoder), [`docs/public-inputs.md`](docs/public-inputs.md)
(what proofs bind to), [`docs/proof-format.md`](docs/proof-format.md) (the
envelope wire format), [`docs/verification.md`](docs/verification.md)
(verifiers, agreement, round trips).

**Privacy & security** — [`docs/privacy.md`](docs/privacy.md) (structural
secret handling), [`docs/security.md`](docs/security.md) (guarantees and
mechanisms), [`docs/threat-model.md`](docs/threat-model.md) (adversaries and
defenses).

**Circuits & backends** — [`docs/circuit-model.md`](docs/circuit-model.md)
(operation circuits, boundaries, measured costs), [`docs/noir.md`](docs/noir.md)
(Noir toolchain split), [`docs/ultrahonk.md`](docs/ultrahonk.md) (real
UltraHonk proving with `bb`).

**Ops & tooling** — [`docs/cli.md`](docs/cli.md) (full command surface),
[`docs/artifacts.md`](docs/artifacts.md) (pinned artifacts and the provider
gate), [`docs/test-vectors.md`](docs/test-vectors.md) (the vector catalog),
[`docs/performance.md`](docs/performance.md) (what each benchmark measures),
[`docs/reproducibility.md`](docs/reproducibility.md) (the pin chain),
[`docs/compatibility.md`](docs/compatibility.md) (versioning policy),
[`docs/deployment.md`](docs/deployment.md) (releases and production gaps).

**Design & roadmap** — [`docs/simulator-integration.md`](docs/simulator-integration.md)
(the simulator boundary and adapter design), [`docs/soroban-verification.md`](docs/soroban-verification.md)
(on-chain verification groundwork), [`docs/testnet.md`](docs/testnet.md)
(optional testnet execution layer).

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
