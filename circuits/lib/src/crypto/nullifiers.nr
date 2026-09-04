//! Nullifiers: double-spend protection for consumed commitments.
//!
//! When a commitment is spent, the circuit publishes a nullifier derived
//! from the commitment and the owner's secret. The ledger stores spent
//! nullifiers, so the same commitment can never be spent twice — but the
//! nullifier reveals nothing about the commitment's amount or the owner's
//! key.

use crate::crypto::hashing;

/// Computes the nullifier for `commitment` under the owner's `secret`.
///
/// Binding the nullifier to both the commitment and the secret means only
/// the true owner can produce it (a stranger cannot, lacking the secret),
/// and once published it uniquely marks that commitment as spent.
pub fn nullifier(secret: Field, commitment: Field) -> Field {
    hashing::pedersen_hash_fields([secret, commitment])
}

/// Computes a token-bound nullifier for `commitment` under `secret`.
///
/// Folding the token into the nullifier means a proof produced for one
/// token's ledger entry cannot be replayed against another token: the
/// nullifier (and therefore the "already spent" record) is per-token.
pub fn nullifier_for_token(secret: Field, commitment: Field, token: Field) -> Field {
    hashing::pedersen_hash_fields([secret, commitment, token])
}

#[test]
fn nullifier_is_deterministic_per_commitment() {
    let c = crate::crypto::commit(50, 9);
    assert(nullifier(1, c) == nullifier(1, c));
}

#[test]
fn nullifier_binds_owner_and_commitment() {
    let c = crate::crypto::commit(50, 9);
    assert(nullifier(1, c) != nullifier(2, c));
    assert(nullifier(1, c) != nullifier(1, crate::crypto::commit(51, 9)));
}

#[test]
fn token_nullifier_is_per_token() {
    let c = crate::crypto::commit(50, 9);
    assert(nullifier_for_token(1, c, 0xaaa) != nullifier_for_token(1, c, 0xbbb));
    assert(nullifier_for_token(1, c, 0xaaa) == nullifier_for_token(1, c, 0xaaa));
}
