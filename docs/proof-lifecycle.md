# Proof Lifecycle

Every proof in Crucible passes through the same stages, from request to
verified artifact. Knowing the lifecycle makes it possible to reason about
where a failure can occur — and the tests in `tests/` exercise every stage.

```text
┌────────────┐   ┌──────────────┐   ┌───────────────┐   ┌──────────────┐
│  REQUEST   │──▶│   WITNESS    │──▶│   PROVING     │──▶│   ENVELOPE   │
│  validated │   │  assembled   │   │  (provider)   │   │   assembled  │
└────────────┘   └──────────────┘   └───────────────┘   └──────────────┘
                                                                 │
                                                                 ▼
┌──────────────┐   ┌──────────────────┐   ┌──────────────┐   ┌──────────┐
│  STORED AS   │◀──│  ROUND-TRIP      │◀──│  VERIFY      │◀──│  LOCAL   │
│  envelope    │   │  verification    │   │  (service)   │   │  verify  │
└──────────────┘   └──────────────────┘   └──────────────┘   └──────────┘
```

## Stage 1 — Request

A `ProofRequest` is created (by the simulator, a scenario, or the CLI). It
carries the operation, circuit, versions, backend, private witness, public
inputs, and — for state-bound operations — a state reference.

**Failure modes**: missing witness, missing public inputs, missing state
binding, unknown operation name, malformed field hex, duplicated witness
names. All caught by structural validation before any backend is consulted.

## Stage 2 — Witness

The private witness bag and the public input bag are assembled into a
`WitnessData`. The assembler enforces that private and public names never
overlap and that required names exist per operation.

**Boundary**: this is the only place private values exist as structured
data. From here they go to the encoder (which writes `Prover.toml` with
restrictive permissions for real circuits) — never to logs, errors, or
serialization.

## Stage 3 — Proving

The provider registered for the request's backend runs. For the mock, this
produces a deterministic envelope binding the request's public context. For
real circuits, this is where nargo/Barretenberg run — consuming the ACIR
artifact and the solved witness.

**Failure modes**: unsupported circuit/version, unavailable artifact,
artifact integrity failure, backend not installed, request targeting the
wrong backend.

## Stage 4 — Envelope assembly

The provider's `ProofResponse` is wrapped into a `ProofEnvelope`: the
versioned, self-describing container that answers *which circuit, which
version, which backend, which verification key, which artifact checksum*
produced the proof.

## Stage 5 — Local verification (round-trip)

`ProverService::prove_and_verify` verifies the proof against its own
response before handing it back. A proof that fails its own round-trip is
surfaced as an error — callers cannot silently proceed with it.

## Stage 6 — Verification service

`crucible-verifier` dispatches a `VerificationRequest` to every verifier
registered for the proof's backend. With multiple verifiers (local and
on-chain), the `VerificationReport` states whether they agree — a
disagreement is a first-class signal, not a swallowed anomaly.

## Stage 7 — Storage

The envelope is the unit of storage, exchange, and cross-language fixture
testing. It serializes to canonical JSON; parsing rejects future versions.

## Failure taxonomy at verification

Verification distinguishes *why* a proof was rejected, so callers and tests
can react precisely:

| Reason | Meaning |
| --- | --- |
| `InvalidProof` | proof bytes failed (tampered, corrupted, wrong key) |
| `PublicOutputMismatch` | outputs differ from what the proof commits to |
| `StateReferenceMismatch` | bound to different state (stale/replay) |
| `WrongVerificationKey` | produced under a different key |
| `CircuitMismatch` / `VersionMismatch` | wrong circuit or version |
| `ArtifactChecksumMismatch` | artifact was tampered or replaced |
| `BackendMismatch` | format does not match this verifier |
| `MissingStateBinding` | binding present on one side only |

A rejected proof is a *valid outcome*, not an error: rejection of tampered,
stale, or misattributed proofs is exactly the behavior Crucible exists to
guarantee.
