# Deployment

What shipping this repository actually means today: the release gate, the
artifact that ships, and what a production deployment would still need.
This is intentionally short and honest — most of this repo is proving
infrastructure, not a deployed service.

## Releases

Cutting a tag (`v*`) triggers `.github/workflows/release.yml`:

1. **The gate** — fmt, clippy, schema validation, and the full workspace
   test suite must pass, then a release build (`cargo build --release`).
2. **The artifact** — the `crucible-prover` CLI binary is attached to the
   tag's GitHub release (creating the release object when a pushed tag has
   none). Only tags whose tree passes the whole gate ship a binary.

The CLI is the deployment surface: circuits, artifact pinning, proving,
verification, vector judging, benchmarking ([docs/cli.md](cli.md)). It is
orchestration only; it performs no key management and holds no secrets.

## What a checkout carries

- **Pinned artifacts** — `artifacts/circuits/<op>/` bytecode + manifests
  are committed and strict-loaded before any proving
  ([docs/artifacts.md](artifacts.md)).
- **Verification keys** — written at runtime into the VK store
  (`artifacts/verification-keys/`, created on demand by `prove` /
  `verify`). Keys are derived by `bb` during proving and resolved by id
  during verification; they are not committed today.
- **Fixtures and vectors** — committed catalog + envelope material used by
  tests and tooling, not shipped to end users.

## Production gaps (by design)

- **On-chain verification is not deployed.** There is no Soroban verifier
  contract integration yet; local verification is real, on-chain
  equivalence is the Soroban workstream ([docs/soroban-verification.md](soroban-verification.md)).
  Stellar labels Confidential Tokens a developer preview — contracts and
  verifier are under audit and not intended for production use
  ([docs/testnet.md](testnet.md)).
- **The exact circuit scheme** is scaffold-shaped until aligned with the
  Confidential Token circuit specification
  ([docs/circuit-model.md](circuit-model.md)); artifacts, keys, and proofs
  produced before that alignment must not be treated as final.
- **No testnet automation** — the optional testnet adapter is a separate
  workstream, kept out of ordinary CI.

## Related

- [docs/reproducibility.md](reproducibility.md) — the pins that make a
  release reproducible.
- [docs/compatibility.md](compatibility.md) — why a release is a bundle of
  agreeing versions, not a single binary.
