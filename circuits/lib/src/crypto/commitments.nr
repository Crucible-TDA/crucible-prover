//! Pedersen commitments over `(amount, blinding)`.
//!
//! A commitment `C = Pedersen([amount, blinding])` hides the amount while
//! binding it: the ledger stores `C`, and a prover later shows knowledge of
//! an opening `(amount, blinding)` without revealing either.
//!
//! # Scope honesty
//!
//! The exact commitment layout of the Confidential Token implementation
//! (inputs hashed, curve, domain tags) must be matched precisely; this
//! module is the single place that alignment lands.

/// Domain tag separating commitment hashes from other hashes.
global COMMITMENT_DOMAIN: Field = 0x636f6d6d; // "comm"

/// Commits to `(amount, blinding)`. Deterministic and hiding.
pub fn commit(amount: Field, blinding: Field) -> Field {
    // The standard library returns an EmbeddedCurvePoint; its x-coordinate
    // is the ledger-facing commitment value.
    std::hash::pedersen_commitment([COMMITMENT_DOMAIN, amount, blinding]).x
}

#[test]
fn commitment_is_deterministic() {
    assert(commit(100, 7) == commit(100, 7));
}

#[test]
fn commitment_hides_value_and_blinding() {
    assert(commit(100, 7) != commit(101, 7));
    assert(commit(100, 7) != commit(100, 8));
}
