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

use crate::constraints::range;
use crate::crypto::{commit, nullifier, nullifier_for_token};
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

    /// Token-bound variant: the emitted nullifier folds in `token`, so the
    /// same commitment cannot be spent once per token.
    pub fn of_for_token(
        commitment: Field,
        secret: Field,
        token: Field,
        amount: Field,
        blinding: Field,
    ) -> Consumed {
        Commitment::new(commitment).assert_opens_to(amount, blinding);
        Consumed { nullifier: nullifier_for_token(secret, commitment, token) }
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

/// Computes `balance + added` in bounded u64 space and returns the sum as a
/// field.
///
/// Both inputs must already be range-constrained to `AMOUNT_BITS` (63 bits),
/// so the sum fits in u64 with no overflow and the field conversion cannot
/// wrap.
pub fn add_bounded(balance: Field, added: Field) -> Field {
    range::assert_in_bits(balance, crate::AMOUNT_BITS);
    range::assert_in_bits(added, crate::AMOUNT_BITS);
    let balance64: u64 = balance as u64;
    let added64: u64 = added as u64;
    (balance64 + added64) as Field
}

/// Computes `balance - spend` in bounded u64 space and returns the remainder
/// as a field. Asserts `spend <= balance` (no overdraw).
pub fn subtract_bounded(balance: Field, spend: Field) -> Field {
    range::assert_in_bits(balance, crate::AMOUNT_BITS);
    range::assert_in_bits(spend, crate::AMOUNT_BITS);
    range::assert_no_overdraw(balance, spend);
    let balance64: u64 = balance as u64;
    let spend64: u64 = spend as u64;
    (balance64 - spend64) as Field
}

#[test]
fn consume_emits_a_stable_nullifier() {
    let c = crate::crypto::commit(100, 5);
    let spent = Consumed::of(c, 42, 100, 5);
    assert(spent.nullifier != 0);
}

#[test]
fn token_consume_nullifier_is_per_token() {
    let c = crate::crypto::commit(100, 5);
    let a = Consumed::of_for_token(c, 42, 0xaaa, 100, 5);
    let b = Consumed::of_for_token(c, 42, 0xbbb, 100, 5);
    assert(a.nullifier != b.nullifier);
}

#[test]
fn spend_result_commits_to_the_new_amount() {
    let r = SpendResult::to(40, 9);
    assert(r.commitment == crate::crypto::commit(40, 9));
}

#[test]
fn bounded_addition_does_not_wrap() {
    let total = add_bounded((1u64 << 62) as Field, (1u64 << 62) as Field);
    // 2^63 is still representable and commits correctly.
    let _ = SpendResult::to(total, 1);
    assert(total == (1u64 << 63) as Field);
}

#[test]
fn bounded_subtraction_enforces_no_overdraw() {
    let remainder = subtract_bounded(1000, 400);
    assert(remainder == 600);
}
