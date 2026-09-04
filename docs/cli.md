# CLI

`crucible-prover` is the command-line face of the proving engine. It is
pure orchestration: every command shells out to the library crates and
keeps no proving logic of its own. Circuit compilation runs through the
`crucible-noir` adapter (never raw `nargo` process calls), proving
through `ProverService`, and verification through the registered
verifiers.

## Building

```bash
cargo build -p crucible-cli
# binary at target/debug/crucible-prover
```

or without building:

```bash
cargo run -q -p crucible-cli -- <command>
```

## Commands

### `circuits`

```bash
crucible-prover circuits list            # the five operation circuits + artifact status
crucible-prover circuits check           # every op must have a compiled, parseable artifact
crucible-prover circuits compile         # compile all five (requires nargo on PATH)
crucible-prover circuits compile transfer
```

`list` reports each operation circuit's ACIR artifact path, size, and
SHA-256. `check` exits non-zero listing every missing or unparseable
artifact. `compile` runs `nargo compile` through the toolchain adapter,
which also enforces the supported nargo major version.

### `artifacts`

```bash
crucible-prover artifacts check              # verify all five pinned artifacts
crucible-prover artifacts check --root /path # against another root
crucible-prover artifacts generate           # re-pin all five from circuits/target
crucible-prover artifacts generate transfer  # re-pin one
```

The proving path only ever consumes the **pinned artifact root**
(`<repo>/artifacts/circuits/<op>/`): each op's compiled bytecode sits
next to a `manifest.json` declaring its SHA-256, and the provider
strict-loads it (see `docs/security.md` G4) before a single byte is
touched. `check` runs that same strict loader over every pinned
artifact and exits non-zero on any integrity problem. `generate`
re-pins artifacts from the current compiled bytecode (run
`circuits compile` first) and is deterministic — identical bytecode
reproduces byte-identical manifests. A circuit change that forgets to
re-pin fails CI via the fresh-compile diff gate.

### `artifacts inspect`

```bash
crucible-prover artifacts inspect          # all five, one report each
crucible-prover artifacts inspect transfer # one op
crucible-prover artifacts inspect transfer --root /path
```

Prints one pinned artifact's full provenance — circuit, circuit and
artifact versions, backend, verification-key id, backend metadata, and
each declared file's role, byte size, and SHA-256 — after running it
through the same strict loader as `check`. A tampered artifact reports
`FAIL` with the reason and still prints the raw manifest for
diagnostics; the exit code is non-zero when anything failed. This is
the `artifact inspect` surface for answering *which circuit produced
this proof* (see `docs/artifacts.md`).

### `witness build`

```bash
crucible-prover witness build transfer \
  --vector test-vectors/transfer/valid/transfer-valid-001.json
crucible-prover witness build transfer \
  --vector test-vectors/transfer/valid/transfer-valid-001.json \
  --out /tmp/transfer.Prover.toml
```

Assembles the circuit witness a vector describes and shows what the
circuit will see: every public value in full, every private value as a
redacted name. With `--out` it writes the Noir `Prover.toml` layout
through `crucible-witness`'s restricted encoder (0600 on Unix) — the
only place private values leave memory — for hand-off to a toolchain
or debugging. The summary path never prints a private value.

### `prove`

```bash
crucible-prover prove transfer \
  --vector test-vectors/transfer/valid/transfer-valid-001.json \
  --backend ultrahonk \
  --out transfer.proof.json
```

Loads a test vector (see `docs/test-vectors.md`), assembles the
[`ProofRequest`] for the chosen backend, and proves through
[`ProverService`]. The local round-trip must pass before an envelope is
written — a proof that fails its own verification is never returned.
For `--backend ultrahonk`, the bytecode is loaded from the pinned
artifact root, not ad-hoc paths; a missing or tampered artifact is
refused before any witness is solved.

- `--backend mock` (default): fast, TEST ONLY, no toolchain required.
- `--backend ultrahonk`: real UltraHonk proofs. Requires `nargo` and
  `bb` on PATH (see `scripts/check-bb.sh`) and compiled bytecode under
  `circuits/target/` (run `circuits compile` first).
- `--out` defaults to `<vector-id>.<backend>.proof.json` in the current
  directory.
- `--vk-store <dir>`: verification-key store for ultrahonk, default
  `<repo>/artifacts/verification-keys` (created on demand).

The printed summary names the request id, circuit version, backend,
verification-key id, public-word count, and the state root the proof is
bound to. The envelope JSON is the unit of storage and exchange.

### `verify`

```bash
crucible-prover verify transfer.proof.json
crucible-prover verify transfer.proof.json --vk-store /path/to/store
```

The envelope is self-describing (backend, verification-key id, public
outputs, state reference), so the command dispatches to the matching
verifier with no user hints: mock envelopes go to `MockVerifier`,
ultrahonk envelopes to `UltraHonkVerifier` resolving the key from the
store. Exit 0 prints the verified summary; every rejection exits 1 with
the reason — tampered bytes, changed public outputs, wrong key, stale
state, or a structurally stripped binding.

### `vectors run`

```bash
crucible-prover vectors run               # judge the whole catalog (mock tier)
crucible-prover vectors run --op transfer # one operation
crucible-prover vectors run --catalog /path/to/vectors
```

A fast, toolchain-free catalog gate mirroring the integration suite's
mock-tier semantics exactly: vectors expected to verify must round-trip
and verify; rejecting vectors must still be well-formed, provable
requests. Non-zero exit on any failure. The nargo-gated circuit tier
(real witness solving against the circuits) runs via
`cargo test -p crucible-tests --test vectors`.

### `benchmark`

```bash
crucible-prover benchmark transfer                 # mock, 3 iterations
crucible-prover benchmark transfer --backend ultrahonk
crucible-prover benchmark transfer --iterations 10 --vector path/to/vector.json
```

Times the proving pipeline phases for one operation's valid witness:
prove, local verify, and envelope serialization, reported as average /
min / max over `--iterations` runs, alongside proof and envelope byte
sizes and the bound state root. The default vector is the operation's
committed valid fixture; pass `--vector` to benchmark another. With
`--backend mock` this measures orchestration overhead only and says so
in its output; with `--backend ultrahonk` it times real UltraHonk
proving through `bb` (requires the toolchains and pinned bytecode, like
`prove`). For in-process, statistically repeated measurements see the
`benches/` harness.

## Path overrides

All defaults resolve relative to the repository root:

| Flag | Default |
|---|---|
| `--circuits <dir>` | `<repo>/circuits` |
| `--catalog <dir>` (`vectors run`) | `<repo>/test-vectors` |
| `--vk-store <dir>` | `<repo>/artifacts/verification-keys` |
| `--root <dir>` (`artifacts`) | `<repo>/artifacts/circuits` |

## Exit codes

| Code | Meaning |
|---|---|
| 0 | success (or `verify`: proof accepted) |
| 1 | runtime error — including verification rejections and failed checks |
| 2 | argument/usage error (clap) |

## Privacy

Witness material flows from the vector file into the request and then,
for ultrahonk, into a `0600` scratch `Prover.toml` via the witness
encoder — it is never echoed in CLI output, and errors never carry
witness values. The proof envelope is public by design (it contains
only the proof, public inputs, and provenance).

## Scope

The CLI does not contain: a wallet, a token contract, a simulator, or a
key-management system. It is the orchestration surface for the
`crucible-prover` pipeline: circuits, artifacts, witnesses, proofs,
verification, and the vector catalog. Soroban on-chain verification
and the testnet adapter are separate workstreams (see
`docs/ultrahonk.md`).