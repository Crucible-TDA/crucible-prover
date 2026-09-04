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

/// Computes a token- and state-bound nullifier for `commitment` under
/// `secret`.
///
/// A 256-bit state root is carried as two field halves (`root_hi`, `root_lo`
/// — the root's two 128-bit halves, each well below the field modulus).
/// Folding both into the nullifier scopes the "already spent" record to the
/// state the operation ran against: the same commitment spent under root A
/// and root B emits different nullifiers, so a proof cannot be replayed
/// across ledger states.
pub fn nullifier_for_token_state(
    secret: Field,
    commitment: Field,
    token: Field,
    root_hi: Field,
    root_lo: Field,
) -> Field {
    hashing::pedersen_hash_fields([secret, commitment, token, root_hi, root_lo])
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

#[test]
fn state_nullifier_is_scoped_to_the_state_root() {
    let c = crate::crypto::commit(50, 9);
    // Same commitment, token, and owner, different state roots.
    let a_hi = 0xababababababababababababababab;
    let a_lo = 0xababababababababababababababab;
    let b_hi = 0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd;
    let b_lo = 0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd;
    assert(
        nullifier_for_token_state(1, c, 0xaaa, a_hi, a_lo)
        != nullifier_for_token_state(1, c, 0xaaa, b_hi, b_lo)
    );
    // Deterministic per root.
    assert(
        nullifier_for_token_state(1, c, 0xaaa, a_hi, a_lo)
        == nullifier_for_token_state(1, c, 0xaaa, a_hi, a_lo)
    );
    // Both halves bind.
    assert(
        nullifier_for_token_state(1, c, 0xaaa, a_hi, a_lo)
        != nullifier_for_token_state(1, c, 0xaaa, a_hi, b_lo)
    );
}
