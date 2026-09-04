//! Hashing helpers used across the circuits.
//!
//! All hashing funnels through these wrappers so a future alignment to a
//! different hash (Poseidon, different domain separation) touches one file.
//!
//! # Scope honesty
//!
//! `pedersen_hash` from the Noir standard library is used for the scaffold.
//! The real Confidential Token specification may require different domain
//! separation or a different hash entirely; alignment is tracked in
//! `docs/noir.md`.

/// Domain tag for generic field hashing.
global HASH_DOMAIN: Field = 0x68617368; // "hash"
/// Domain tag for key derivation (addresses). Distinct from HASH_DOMAIN so
/// an address hash can never collide with a generic hash of the same inputs.
global KEY_DOMAIN: Field = 0x6b6579; // "key"

/// Hashes a fixed-size field array with the standard Pedersen hash.
pub fn pedersen_hash_fields<let N: u32>(inputs: [Field; N]) -> Field {
    std::hash::pedersen_hash(inputs)
}

/// Derives a public key hash from a secret key (address derivation).
///
/// Domain-separated from generic hashing via `KEY_DOMAIN`, so hashing a
/// secret as a *key* yields a different value than hashing it as *data*.
pub fn key_hash(secret: [Field; 1]) -> Field {
    std::hash::pedersen_hash([KEY_DOMAIN, secret[0]])
}

#[test]
fn hash_is_deterministic() {
    assert(pedersen_hash_fields([1, 2]) == pedersen_hash_fields([1, 2]));
    assert(pedersen_hash_fields([1, 2]) != pedersen_hash_fields([1, 3]));
}

#[test]
fn key_hash_is_domain_separated_from_generic_hash() {
    // Different domain tags must yield different hashes for the same data.
    assert(key_hash([5]) != pedersen_hash_fields([42, 5]));
    assert(key_hash([5]) != pedersen_hash_fields([0x68617368, 5]));
}

#[test]
fn key_hash_is_deterministic() {
    assert(key_hash([5]) == key_hash([5]));
    assert(key_hash([5]) != key_hash([6]));
}
