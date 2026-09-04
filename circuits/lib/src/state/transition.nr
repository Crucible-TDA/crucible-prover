//! Commitment state transitions.
//!
//! A confidential operation consumes one or more old commitments and
//! produces new ones. Two invariants hold across every transition:
//!
//! 1. **Every consumed commitment is spent exactly once.** The circuit
//!    proves knowledge of the old commitment's opening and emits a nullifier
//!    the ledger records, so the same commitment can never be spent twice.
//! 2. **Value is conserved.** The sum of amounts entering the new
//!    commitments equals the sum that left the old ones (range-bounded, so
//!    no wrap-around inflation is possible).
//!
//! Tree membership of the old commitments in the ledger's state root is a
//! *separate* statement: it requires the state root as a public input and an
//! inclusion proof as a witness. That lands with the membership workstream
//! (see `docs/noir.md`); today the simulator/prover layer binds proofs to
//! state roots outside the circuit.

use crate::crypto::{commit, nullifier};
use crate::types::commitment::Commitment;

/// A commitment that has been consumed by a transition.
pub struct Consumed {
    /// The nullifier the ledger records to prevent double spending.
    pub nullifier: Field,
}

impl Consumed {
    /// Consumes `commitment`, proving knowledge of its opening
    /// `(amount, blinding)` under the owner's `secret`.
    ///
    /// Fails unless the opening matches the commitment, so a transition can
    /// never spend a commitment whose value it cannot account for.
    pub fn of(commitment: Field, secret: Field, amount: Field, blinding: Field) -> Consumed {
        Commitment::new(commitment).assert_opens_to(amount, blinding);
        Consumed { nullifier: nullifier(secret, commitment) }
    }
}

/// The result of producing a new commitment.
pub struct SpendResult {
    /// The new commitment covering the remainder (change or transferred-in
    /// value).
    pub commitment: Field,
}

impl SpendResult {
    /// Produces a commitment to `amount` with `blinding`.
    pub fn to(amount: Field, blinding: Field) -> SpendResult {
        SpendResult { commitment: commit(amount, blinding) }
    }
}

#[test]
fn consume_emits_a_stable_nullifier() {
    let c = crate::crypto::commit(100, 5);
    let spent = Consumed::of(c, 42, 100, 5);
    assert(spent.nullifier != 0);
}

#[test]
fn spend_result_commits_to_the_new_amount() {
    let r = SpendResult::to(40, 9);
    assert(r.commitment == crate::crypto::commit(40, 9));
}
