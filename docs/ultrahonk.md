# UltraHonk backend: real proving with Barretenberg

How `crucible-prover` generates and verifies real UltraHonk proofs through
the Barretenberg binary (`bb`), and what this repository has validated about
that pairing.

## Toolchain pairing (validated)

| Component | Version | Role |
| --- | --- | --- |
| `nargo` | `1.0.0-beta.26` | compile circuits, solve witnesses |
| `bb` | `6.0.0-nightly.20260903` | UltraHonk prove / verify |

This exact pairing is what the repository is **validated against**: the
compatibility matrix in `crates/ultrahonk/src/backend.rs` (`BACKEND_COMPAT`),
the `TESTED_BB_VERSION` pin in the crate root, and the CI circuits job (which
installs `bb` via `bbup` at that version) all agree on it. Proofs are only
reproducible when the backend version is known: an UltraHonk proof produced
by one Barretenberg version may not verify with another.

> **Installing `bb`**: `bbup` normally resolves the right `bb` for your
> `nargo` automatically, but its mapping (`bb-versions.json`) can lag new
> Noir releases. When that happens, pin explicitly:
>
> ```bash
> curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/next/barretenberg/bbup/install | bash
> bbup -v 6.0.0-nightly.20260903 --no-modify-path
> ```
>
> `scripts/check-bb.sh` reports the installed version and repeats these
> instructions.

## The tool split (why `bb` exists at all)

Modern Noir (1.0.0-beta.x) removed proving from `nargo`:

| Stage | Tool | Produces |
| --- | --- | --- |
| Compile | `nargo compile` | ACIR bytecode JSON |
| Execute | `nargo execute` | solved witness (`.gz`) from `Prover.toml` |
| Prove | `bb prove` | UltraHonk proof + verification key |
| Verify | `bb verify` | accept/reject against the VK |

`nargo test` output is an in-process interpreter run and is **not** a proof;
only `bb` produces cryptographic evidence.

## The `bb` CLI surface (what this adapter drives)

The validated `bb` exposes a small, stable command surface (no more
`prove_ultra_honk` / `write_vk_ultra_honk` subcommands):

```bash
bb prove -b <bytecode.json> -w <witness.gz> -o <dir> --write_vk --output_format json
bb verify -p <dir>/proof.json -k <dir>/vk.json -i <dir>/public_inputs.json
```

- The scheme for Noir ACIR is **`ultra_honk`**.
- With `--output_format json`, every artifact is self-describing and carries
  `scheme`, `bb_version`, and the backend-native verification-key digest
  (`vk_hash` / `hash`):
  - `proof.json` — `{ proof: [field words…], vk_hash, bb_version, scheme }`
  - `public_inputs.json` — `{ public_inputs: [field words…], bb_version, scheme }`
  - `vk.json` — `{ vk: [field words…], hash, bb_version, scheme }`
- Field words are `0x`-prefixed 32-byte big-endian hex. Public inputs are
  listed in circuit order: `pub` parameters first, then returned values.
- `bb verify` exits 0 on acceptance and non-zero on rejection; a rejected
  proof prints e.g. `UltraVerifier: verification failed at reduction step`.

## Repository mapping

- `crates/ultrahonk/src/toolchain.rs` — `BbToolchain`: locating `bb`
  (`BB_BIN` override), parsing `bb --version`, major-version floor
  (pre-2026 CLI generations are rejected).
- `crates/ultrahonk/src/exec.rs` — `prove()` / `verify()`: process
  execution, JSON artifact parsing, provenance validation (scheme must be
  `ultra_honk`, `bb_version` must be present, and the digest a proof embeds
  must equal the digest of the VK written alongside it). A proof that fails
  verification is an outcome, not an error.
- `crates/ultrahonk/src/backend.rs` — the compatibility matrix.
- `crates/ultrahonk/src/calldata.rs` — public-input encoding for an on-chain
  verifier (candidate layout; calibration against a deployed Soroban
  verifier is deferred to the Soroban batch).

Witness and bytecode are referenced **by path only**; the adapter never
reads witness material into memory, and errors carry paths and exit codes,
never values or raw stderr.

## Live test coverage

`tests/tests/ultrahonk.rs` runs real cryptography end to end, gated on both
`nargo` and `bb` being on `PATH`:

- witnesses solved from the committed vector catalog against the real
  `register` / `transfer` circuit packages;
- `bb prove` → `bb verify` round trips;
- **binding**: a register proof's single public input must equal the
  committed account address; a transfer proof exposes exactly seven public
  words — token, sender, recipient, old commitment, then the three returned
  values — checked word by word against the fixture;
- **rejection**: tampered proofs, tampered/wrong verification keys, and
  proofs submitted against changed public inputs all fail verification.

These are the cryptographic counterparts of the wrong-context rejections
the mock backend can only simulate, and they run in the CI circuits job
where the validated toolchain pair is installed.

## What is deliberately not here yet

- Wiring this file-level proving behind the `ProofProvider`/`Verifier`
  traits (verification-key store + witness-bag bridge + artifact resolver)
  is the application boundary, still to land.
- On-chain verification (Soroban UltraHonk verifier, calldata calibration,
  `verifier_target` selection) is the Soroban adapter's job.
- The circuits in this repository encode the *shape* of Confidential Token
  semantics; exact commitment layout and key derivation must be aligned with
  the real OpenZeppelin spec before any production use.
