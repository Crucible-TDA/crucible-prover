# Contributing to crucible-prover

Thanks for contributing to the Crucible proof engine.

## Code of conduct

All contributors must follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Repository map

- `interfaces/` — the stable contracts (`ProofProvider`, `Verifier`,
  request/response types). This crate must stay dependency-light; every
  other crate builds on it.
- `crates/` — the Rust engine: witness management, proof types, artifact
  integrity, prover orchestration, backends (Noir, UltraHonk), verification.
- `adapters/` — bridges to external systems (simulator, Soroban, testnet).
- `circuits/` — the Noir workspace (shared library, production circuits,
  measurement gadgets). Noir is a separate toolchain from Cargo.
- `schemas/`, `test-vectors/` — cross-language fixtures.
- `cli/` — orchestration only. No proving logic lives here.
- `docs/` — architecture and design documents.

## Development setup

Install the pinned toolchains:

```bash
# Rust (pinned in rust-toolchain.toml)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Noir (see scripts/setup-noir.sh)
curl -L https://raw.githubusercontent.com/noir-lang/noirup/main/install | bash
export PATH="$HOME/.noirup/bin:$PATH"
noirup
```

## Before opening a PR

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. If you touched circuits: `scripts/check-circuits.sh`
5. If you touched serialization or proof formats: extend the matching JSON
   schema in `schemas/` and regenerate affected test vectors.

Tests that touch private witness material must assert the material never
surfaces in `Debug`/`Display` output, errors, or logs (see the witness
leakage suites under `tests/`).

## Commit conventions

- One logical improvement per commit; do not bundle unrelated changes.
- Detailed commit messages explaining the *what* and the *why*.
- Reference the security implications of your change in the message when
  the change touches witness handling, verification, or artifacts.

## Where to start

See `docs/architecture.md` and the issue templates under `.github/ISSUE_TEMPLATE/`.
Good first issues are tagged `good first issue`; circuit and test-vector work
does not require deep Rust knowledge, while `prover-core` and `witness` work
requires care with the privacy boundary.
