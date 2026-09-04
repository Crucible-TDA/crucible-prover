# Soroban verification

The design for verifying UltraHonk proofs through a Soroban verifier
contract — and why local verification must never be assumed equivalent to
on-chain verification without testing. **Status: design + groundwork only;
the on-chain flow is not implemented.**

## Why it matters

Crucible's core promise is that local and on-chain verification are tested,
not assumed, to agree ([docs/verification.md](verification.md)). The
current Confidential Token stack verifies UltraHonk proofs on-chain; the
two failure classes this workstream exists to catch:

1. **Encoding mismatch** — the public inputs must reach the verifier
   contract in exactly the byte order and width it expects. Getting this
   wrong is a silent "verification failed" on-chain even though local
   verification passed.
2. **Calldata/verifier drift** — the deployed verifier's layout is defined
   by the verifier contract, which is outside this repository until the
   Soroban batch lands.

## Groundwork already in place

- **`CalldataEncoder`** (`crates/ultrahonk/src/calldata.rs`) packs a
  circuit's public inputs — in ABI declaration order, 32-byte big-endian
  field elements — into `[version][count][inputs…]`. It is deterministic,
  round-trips through `decode`, and carries a format-version byte so
  calibration can ship as a new version without breaking stored fixtures.
  It is explicitly a *candidate* layout until validated against the real
  verifier contract.
- **Cross-verifier agreement** — `VerificationService` already supports
  registering several verifiers per backend; a `soroban` registration
  serving the `ultrahonk` backend is the intended shape, and
  disagreement would surface in the `VerificationReport`
  ([docs/verification.md](verification.md)).
- **Envelope self-description** — proofs carry backend, vk id, public
  outputs, and state reference, so a Soroban adapter can build its
  transaction from the envelope with no user hints
  ([docs/proof-format.md](proof-format.md)).

## What remains (the Soroban batch)

1. The UltraHonk verifier contract address/ABI for the target network.
2. An `adapters/soroban` crate: envelope → calldata → Soroban transaction →
   verifier call, registered as a verifier alongside `local`.
3. Live agreement tests: the same proof through local `bb verify` and the
   on-chain verifier, asserting agreement.
4. **Membership**: state-bound operations consume commitments; proving the
   consumed commitment is in the ledger's commitment tree needs Merkle
   membership (inclusion proofs) in the circuits — currently out of scope
   ([docs/circuit-model.md](circuit-model.md)).

Stellar labels Confidential Tokens a developer preview — the contracts and
verifier are under audit and not intended for production use
([docs/testnet.md](testnet.md), [docs/deployment.md](deployment.md)).
