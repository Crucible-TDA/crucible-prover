# Artifact Management

The proving path must always know exactly **which circuit produced this
proof**. `crucible-prover` answers that with a *pinned artifact root*: every
compiled circuit sits in the repository next to a manifest declaring each
file's SHA-256, and no component proves against ad-hoc bytecode.

See [`security.md`](./security.md) G4 and threat-model A4 (the artifact
swapper) for the adversarial framing.

## Layout

```text
artifacts/
└── circuits/
    ├── register/
    │   ├── manifest.json        # declares files + SHA-256s, backend, versions
    │   └── register.json        # compiled ACIR bytecode
    ├── deposit/   …
    ├── merge/     …
    ├── transfer/  …
    └── withdraw/  …
```

Verification keys produced during proving are written to
`artifacts/verification-keys/` by the `VkStore`, keyed by a digest of
circuit id + circuit version + artifact checksum — never by an untrusted
proof-supplied name.

## The manifest

Each `manifest.json` records the circuit identity (`circuit`,
`circuit_version`, `artifact_version`), the backend it was built for, and
one entry per file:

```json
{
  "manifest_version": 1,
  "circuit": "transfer",
  "circuit_version": "0.1.0",
  "artifact_version": "0.1.0",
  "backend": "ultrahonk",
  "files": [
    { "path": "transfer.json", "sha256": "7cf88c43…", "kind": "acir" }
  ],
  "backend_metadata": { "generated_by": "crucible-prover/0.1.0" }
}
```

Generation is deterministic: identical bytecode reproduces byte-identical
manifests, which is what makes the CI freshness gate (below) possible.

## The provider gate

`UltraHonkProvider::generate` refuses to prove until the artifact passes
[`crucible-artifacts`]' strict loader:

1. the manifest must parse and its paths must stay inside the artifact
   directory (no traversal);
2. every declared file must exist and match its SHA-256 byte-for-byte;
3. no undeclared file may be present (strict mode).

Only then is a witness solved and `bb` invoked. Failures map to
`ProviderError::ArtifactIntegrity` (tampered bytes, extra files) or
`ArtifactUnavailable` (missing manifest/bytecode) — always *before* any
proving work, so a swapped artifact is rejected without touching secrets or
backend state.

## CLI

```bash
crucible-prover artifacts check              # verify all five pinned artifacts
crucible-prover artifacts generate           # re-pin from circuits/target (deterministic)
crucible-prover artifacts generate transfer  # re-pin one op
```

- `check` runs the same strict loader the provider runs and exits non-zero
  listing every problem.
- `generate` copies `<circuits>/target/<op>.json` into the pinned root next
  to a freshly computed manifest. Requires compiled bytecode
  (`crucible-prover circuits compile`).

## Keeping artifacts honest

A circuit-source change that forgets to re-pin its artifact is a
stale-bytecode hazard: witnesses would be solved against new source while
proofs verify against old circuits. CI therefore enforces two gates:

- `artifacts check` — the committed artifacts are self-consistent
  (manifest matches files);
- a **fresh-compile determinism gate** — `artifacts generate` into a temp
  root, then `diff -r` against the committed `artifacts/circuits`. Any
  circuit change that didn't re-pin fails the build.

The toolchain is pinned to match the committed artifacts
(`noirup -v 1.0.0-beta.26`, `bbup -v 6.0.0-nightly.20260903`) so fresh
compiles are reproducible.

## Tested by

- the live `tests/tests/artifacts.rs` suite — attacks a copy of the pinned
  artifact: single-byte bytecode flip, missing manifest, missing bytecode,
  planted extra file; all must be rejected before proving, and an intact
  pinned register artifact must still prove a real UltraHonk proof;
- `tests/security/artifact_tampering.rs` (mock-tier) and the
  `crucible-artifacts` unit suite (traversal manifests, checksum edits).

[`crucible-artifacts`]: ../crates/artifacts