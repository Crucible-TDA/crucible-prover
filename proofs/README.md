# proofs

Committed proof material: envelope fixtures used to pin the wire format
across tooling versions and consumers.

## Layout

```text
proofs/
├── README.md
└── fixtures/
    ├── valid/
    │   ├── register-valid-001.mock.proof.json
    │   ├── deposit-valid-001.mock.proof.json
    │   ├── merge-valid-001.mock.proof.json
    │   ├── transfer-valid-001.mock.proof.json
    │   └── withdraw-valid-001.mock.proof.json
    └── invalid/
        └── transfer-tampered-proof-001.json   (one byte flipped)
```

## What a fixture is

Every file is a serialized [`ProofEnvelope`] (envelope format **v1** — see
`crates/proof-types`): a self-describing container naming the operation,
circuit + version, backend, verification-key id, artifact checksum, public
outputs, and the state root the proof is bound to. Because the envelope is
the unit of storage and exchange, committing a few of them pins:

- **serialization compatibility** — the v1 JSON layout is a cross-version
  contract; old fixtures must keep parsing as the format evolves
  (envelope parsing rejects future versions instead of guessing);
- **verification behavior** — valid fixtures must verify against the mock
  backend with no hints, and the tampered fixture must be rejected;
- **provenance shape** — what a real proof's metadata looks like, so any
  consumer (CLI, scenario suites, non-Rust harnesses) can be checked
  against concrete material rather than imagined shapes.

## Privacy

Proof envelopes are **public by design**: they contain the proof bytes,
public outputs, and provenance — never private witness values. These
fixtures are mock proofs of the catalog's synthetic vectors (sample keys
only); they are safe to commit, which is exactly why the envelope format
draws the public/private line where it does.

## Regenerating

Fixtures are produced by the CLI against the committed vector catalog and
are **deterministic** (same vector + same mock key ⇒ same bytes), so
regeneration should be a no-op diff unless the envelope format or a vector
changed:

```bash
bash scripts/generate-proof-fixtures.sh
```

Run `git diff --stat proofs/` after regenerating: any change means the
format or catalog moved, and the fixture commit should land with that
change.

## Judging

`tests/tests/proof_fixtures.rs` loads every committed fixture: valid ones
must round-trip through envelope JSON and verify against the mock verifier;
the invalid one must be rejected. This is the regression net that keeps the
committed material honest as the workspace evolves.

[`ProofEnvelope`]: ../crates/proof-types/src/common.rs
