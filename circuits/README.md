# circuits

The Noir circuit workspace of `crucible-prover`. This is a **Noir workspace**,
independent from the Cargo workspace: `nargo` compiles these packages into
ACIR artifacts, and the Rust crates consume those artifacts through
`crucible-noir` and `crucible-artifacts`.

## Packages

| Package | Type | Proves |
| --- | --- | --- |
| `lib` | lib | shared primitives: types, crypto, constraints, state helpers |
| `register` | bin | account registration (key ownership) |
| `deposit` | bin | adding confidential value to an owned commitment |
| `merge` | bin | consolidating two owned commitments into one |
| `transfer` | bin | moving confidential value between accounts |
| `withdraw` | bin | redeeming confidential value out of the domain |
| `gadgets/*` | bin | tiny circuits measuring one primitive's constraint cost |

## Layout

```text
circuits/
├── Nargo.toml          workspace manifest
├── lib/                shared Noir library (consumed by every circuit)
├── register/  deposit/ merge/  transfer/  withdraw/
│   ├── Nargo.toml
│   ├── src/main.nr     the circuit's main() — one proving entry point
│   └── testdata/       Prover.toml inputs used by scripts and vectors
└── gadgets/            one package per primitive for nargo info measurement
```

## Toolchain

The workspace is developed against **nargo 1.0.0-beta.26** (see
`rust-toolchain.toml`? no — see `scripts/check-circuits.sh`). The compiler
version is declared in every package's `Nargo.toml`; `nargo` enforces it.

## Commands

```bash
# Compile every package (ACIR → target/<pkg>.json)
nargo compile --workspace --force

# Run the unit tests embedded in each package (#[test])
nargo test --workspace

# Report circuit metrics (ACIR/Brillig opcodes) per package
nargo info --workspace
```

## Scope honesty

These circuits encode the *shape* of the Confidential Token operations —
Pedersen commitments over (amount, blinding), range-bounded amounts,
key-derived nullifiers, ownership via secret-key hash — so the proving
architecture can be exercised end to end. The exact public-input/witness
boundary and cryptographic scheme must be aligned with the target
Confidential Token circuit specification; `docs/noir.md` tracks what is
scaffold and what must be verified against the real circuits.
