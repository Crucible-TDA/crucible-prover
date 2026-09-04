# Noir toolchain integration

How `crucible-prover` uses the Noir toolchain: which binaries exist today,
what the Rust adapters own, and which boundaries the circuit workspace is
developed against.

## Toolchain split (read this first)

Modern Noir (1.0.0-beta.x, the version this repo is developed against)
split proving out of `nargo`:

| Stage | Tool | Produces |
| --- | --- | --- |
| Compile | `nargo compile` | ACIR circuit artifacts (`target/<pkg>.json`) |
| Execute | `nargo execute` | a solved witness from `Prover.toml` inputs |
| Metrics | `nargo info` | per-function ACIR/Brillig opcode counts |
| Unit tests | `nargo test` | in-circuit `#[test]` execution |
| Prove | `bb prove` (Barretenberg) | the actual ZK proof |
| Verify | `bb verify` (Barretenberg) | proof acceptance against a VK |

`nargo prove` / `nargo verify` no longer exist as subcommands. The Rust
adapter split follows the tool split:

- `crucible-noir` ends at **artifacts + witnesses** (compile, execute, info,
  artifact parsing, version checks). It never shells out to a prover.
- Real UltraHonk proving belongs to the Barretenberg `bb` backend and the
  `crucible-ultrahonk` crate; it is wired behind the same `ProofProvider`
  seam as everything else, and is out of scope until the batch that
  introduces the `bb` integration.

Requiring `bb` is also why `nargo test` output — which runs the circuits on
an in-process interpreter — is not a substitute for real proofs, only for
witness-solvability checks.

## Repository layout

The circuit workspace is **independent of the Cargo workspace**, mirroring
the OpenZeppelin `stellar-contracts` model: `circuits/` is its own `nargo`
workspace and the Rust crates consume its outputs. See `circuits/README.md`
for the package layout and commands.

## Committed artifacts vs. build output

`circuits/target/` holds compiled ACIR and solved witnesses; it is
gitignored and rebuilt by CI. What *is* committed:

- the Noir source (`src/`),
- one synthetic valid test vector per operation circuit
  (`<op>/testdata/Prover.toml`), re-included in `.gitignore` deliberately —
  these are public fixtures with sample keys only.

Every other `Prover.toml` is gitignored: witness files can carry real
private values and must never land in Git history. This is the same rule
the `crucible-witness` crate enforces for Rust-side witnesses.

## Version pinning

Every package declares `compiler_version = ">=1.0.0"` in its `Nargo.toml`
and `nargo` enforces it. `scripts/check-circuits.sh` reports the toolchain
version; CI installs via `noirup` and runs the circuit suites.

## Nargo-driven integration tests

`crates/noir` has integration tests that run a live `nargo compile` /
`nargo execute` against a scratch project to validate the adapter's CLI
surface, artifact parsing, and witness plumbing. Those tests are the only
place in the Rust workspace that requires `nargo` on PATH; they are gated
accordingly (see `scripts/check-circuits.sh`).
