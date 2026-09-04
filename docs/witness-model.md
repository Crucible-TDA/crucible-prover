# Witness model

How a proof request's inputs are split, assembled, validated, and encoded
for the toolchain — and why the private/public boundary is structural, not
a convention.

## The boundary

Every operation circuit has two kinds of inputs:

```text
PRIVATE WITNESS   values only the prover knows (secrets, amounts, blindings)
PUBLIC INPUT      context everyone can see (addresses, commitments, state root)
```

In code the split is enforced by **two bag types** that cannot be confused:

- `PrivateWitnessBag` holds `SecretValue`s. `SecretValue` implements **no
  `Debug`, no `Display`, no `Serialize`** — it cannot be formatted, logged,
  or serialized by accident. The bag exposes names and counts only.
- `PublicInputBag` holds `FieldValue`s (canonical hex), which are public by
  design and safe to log and fixture.

A request carries both sides plus the circuit identity it belongs to
(`interfaces::ProofRequest`). Because the private bag cannot serialize,
`ProofRequest` has no `Serialize` either; its only JSON path is the redacted
view ([`ProofRequest::redacted`]).

## Assembly

`crucible-witness` owns the seam between a request and the toolchain:

- [`builder`] — `WitnessAssembler` merges public and private bags into one
  `WitnessData`, rejecting **name overlap** between the two sides and
  missing required names. Required names are caller-supplied because the
  exact circuit interface is defined per circuit (`interfaces::circuit::
  expectations` pins the per-operation public surface).
- [`validation`] — structural rules every witness must satisfy before it
  can be encoded.
- [`decoder`] — parses circuit **public outputs** back into an `OutputBag`.
  It never reconstructs private values.

## Encoding (the single escape hatch)

Private values leave memory in exactly one place:
[`encoder::write_prover_toml`], which writes the Noir `Prover.toml` layout
as `0x`-prefixed hex with **0600 permissions** (Unix). Two rules make this
safe:

1. Nothing else in the workspace prints a secret value: `WitnessData` and
   both bags implement `Debug` as redacted views, error messages carry
   paths and counts, and toolchain stderr is never echoed into errors
   (compiler diagnostics can embed source snippets).
2. `0x`-prefixing is not cosmetic: Noir's witness parser treats a bare
   string as **decimal**, so an unprefixed `ab` fails to parse and a bare
   `1234` silently means decimal 1234. The encoder guarantees hex.

The assembly step is also exposed directly — `crucible-prover witness
build` builds a witness from a test vector and either prints a redacted
summary or writes a `Prover.toml` for hand-off to a toolchain.

## Where this fits

```text
ProofRequest ──► WitnessAssembler ──► WitnessData
                                        │ encoder (0600)
                                        ▼
                                   Prover.toml ──► nargo execute (witness solve)
                                        │
                                        ▼
                        bb prove (consumes witness + pinned bytecode)
```

`crucible-vectors` maps catalog JSON onto the same bags so fixtures
exercise the identical code path as live proving. See
[docs/proving-model.md](proving-model.md) for the request/response model and
[docs/privacy.md](privacy.md) for the guarantees that follow from this
design.
