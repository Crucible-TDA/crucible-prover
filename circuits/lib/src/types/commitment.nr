//! Confidential value commitments.
//!
//! A commitment hides a balance behind a Pedersen commitment over
//! `(amount, blinding)`; the ledger stores commitments and never sees the
//! amount. This module defines the ledger-facing wrapper type. The
//! construction and opening rules live in `crate::crypto::commitments`.

use crate::crypto::commitments;

/// A ledger-stored confidential commitment (hides its amount and blinding).
pub struct Commitment {
    /// The commitment field value.
    pub value: Field,
}

impl Commitment {
    /// Wraps an existing commitment value.
    pub fn new(value: Field) -> Commitment {
        Commitment { value }
    }

    /// Creates a commitment to `amount` with `blinding`.
    pub fn to(amount: Field, blinding: Field) -> Commitment {
        Commitment { value: commitments::commit(amount, blinding) }
    }

    /// The underlying field value.
    pub fn inner(self) -> Field {
        self.value
    }

    /// Asserts that this commitment opens to `(amount, blinding)`.
    ///
    /// Used by circuits that consume existing commitments as public inputs:
    /// the prover must show the (private) opening matches the public
    /// commitment before any state transition is accepted.
    pub fn assert_opens_to(self, amount: Field, blinding: Field) {
        assert(
            self.value == commitments::commit(amount, blinding),
            "commitment does not open to the provided witness",
        );
    }
}

#[test]
fn commitment_wraps_and_opens() {
    let c = Commitment::to(100, 0xbeef);
    c.assert_opens_to(100, 0xbeef);
}
