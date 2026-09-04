# Proof format

The wire representation of a proof: what is stored and exchanged, who owns
the cryptographic encoding, and how the format stays versioned and
traceable.

## ProofEnvelope (format v1)

The unit of storage and exchange is the versioned
[`ProofEnvelope`](../crates/proof-types/src/common.rs) (current version
**1**). It is self-describing — a consumer can decide how to verify and
reproduce a proof without guessing at provenance:

```text
ProofEnvelope {
    version            envelope wire-format version (reject > known)
    operation          register | deposit | merge | transfer | withdraw
    circuit            circuit id
    circuit_version    circuit the proof is valid for
    backend            mock | ultrahonk | …
    proof              ProofBlob { format tag, hex bytes }
    public_outputs     ordered bag of bound public values
    verification_key_id  which key the proof must be checked against
    artifact_checksum  SHA-256 of the artifact that produced the proof
    state_reference    (root, sequence) when the proof is state-bound
    metadata           request_id + producer label
}
```

**The cryptographic encoding lives in the backend, never here.** The
envelope names the encoding via a format tag; it does not invent one.
Current tags: `mock-envelope-v1` (test-only deterministic envelopes) and
`ultrahonk-v1` (Barretenberg UltraHonk proofs).

## Rules

- **Proof bytes are hex** in the envelope (`ProofBlob`), for stable JSON
  serialization across languages and tools.
- **Parsing rejects unknown future versions** (`version > 1` fails with
  `UnsupportedVersion`): old tooling can never silently misinterpret newer
  proofs. Envelope JSON serialization is deterministic.
- **Backend identity and versions are carried**, so a proof is traceable to
  its circuit, toolchain pairing, and artifact checksum
  ([docs/compatibility.md](compatibility.md), [docs/reproducibility.md](reproducibility.md)).
- **Every proof answers "which circuit produced this?"** via the pinned
  artifact's manifest ([docs/artifacts.md](artifacts.md)), including the
  artifact checksum that also discriminates the verification-key id.

## Committed material

Serialized v1 envelopes are committed under `proofs/fixtures/` (one valid
mock envelope per operation plus a tampered fixture) and judged by
`tests/tests/proof_fixtures.rs` — a regression net that pins the v1 JSON
layout and the verification contract against real bytes
([proofs/README.md](../proofs/README.md)).

## Related

- [docs/public-inputs.md](public-inputs.md) — what the envelope's public
  outputs mean.
- [docs/verification.md](verification.md) — how an envelope is turned back
  into a verification request.
- [docs/compatibility.md](compatibility.md) — versioning policy across the
  envelope, manifests, and toolchains.
