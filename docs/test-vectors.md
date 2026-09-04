# Test vectors

The `test-vectors/` directory holds the **cross-language vector catalog**: one
JSON document per scenario, encoding what a proving implementation must
accept and what it must reject, in a format no Rust code is required to read.
The same file drives the Rust runner, the Python schema checker, and any
future non-Rust consumer (a JS/Soroban harness, the scenario suites, `bb`
backend tests).

## What a vector says

Each file matches `schemas/test-vector.schema.json` and carries:

- **`operation` / `circuit` / `circuit_version`** — what is being proven and
  by which circuit version.
- **`category`** — `valid`, or a reject category such as `wrong-owner`,
  `insufficient-balance`, `invalid` (opening mismatch), `stale-state`,
  `malformed-proof`, `replay`. The category is the semantic contract: a
  `wrong-owner` vector must fail *because the ownership assertion fails*, not
  for any other reason.
- **`witness`** — the circuit inputs: public entries plus private values.
  Values are **canonical lowercase hex** (no `0x`, no leading zeros), the
  same format `FieldValue`/`SecretValue` enforce. Private values are the
  circuit's private `main` parameters (sample keys only — see Privacy below).
- **`expected_public_outputs`** — the exact values the circuit reports when
  the witness solves, in circuit return order. Captured from real `nargo
  execute` runs, so a fixture that drifts from the circuits fails loudly.
- **`state_reference`** — the state context the proof binds to (`null` for
  operations that are not state-bound).
- **`expect_verification`** — `true` for `valid`, `false` for reject
  categories.

## Directory layout

```text
test-vectors/
├── register/  valid/  wrong-owner/
├── deposit/   valid/  invalid/
├── merge/     valid/  invalid/
├── transfer/  valid/  invalid/  insufficient-balance/  wrong-owner/
├── withdraw/  valid/  insufficient-balance/  wrong-owner/
└── cross-operation/   (future: replay/stale-state against proofs)
```

Directory names mirror the `category` field; file ids are globally unique
(`<op>-<category>-<n>`).

## How vectors are judged (the runner)

`tests/tests/vectors.rs` executes every catalog entry in two tiers:

1. **Mock tier** (always runs): every vector must produce a structurally
   valid `ProofRequest`; `valid` vectors must round-trip through the mock
   stack (prove → verify). The mock is semantically blind, so reject
   categories are *not* judged here — only their well-formedness is pinned.
2. **Circuit tier** (runs when `nargo` is on `PATH`): each vector's witness is
   written as a `Prover.toml` and executed against the real Noir circuit
   package. `valid` vectors must solve and report exactly the fixture's
   expected outputs; reject vectors must **not** solve.

The two-tier split is deliberate and honest: the mock proves a request is
expressible, the circuit proves its witness is (or is not) satisfiable. A
vector failing the wrong tier is a catalog bug.

## Generating vectors

Vectors are generated against real circuit executions, not by hand:

1. `scripts/generate-test-vectors.sh` runs each committed circuit
   `Prover.toml` and fails if any no longer solves (drift canary).
2. The JSON `expected_public_outputs` in the catalog are copied from real
   `nargo execute` output (the `Circuit output:` line), and the Rust runner's
   circuit tier re-checks them on every run.

Add a vector by: computing the witness against the circuit, executing it
through nargo to capture outputs, and committing the JSON. The runner
validates that you got both tiers right.

## Schema conformance

`scripts/check-schemas.py` validates **every** file under `test-vectors/`
against `test-vector.schema.json` as part of `scripts/check.sh`, so JSON-level
conformance is enforced without building Rust.

## Privacy

Vector private values are synthetic sample material — the same keys
(`0x1234`, `0x5678`) that appear in the circuit sources and their committed
`testdata/Prover.toml` files. They carry no real secrecy. Real witness
material must never be committed as a vector: the JSON loader rejects
non-canonical hex, and the `witness` encoder crate keeps live material out of
files entirely.
