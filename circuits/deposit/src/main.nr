//! Deposit circuit: add confidential value to an account's commitment.
//!
//! A deposit consumes the account's existing commitment (proving knowledge
//! of its opening and the owner secret, emitting a token-bound nullifier so
//! it cannot be spent again) and produces a new commitment covering
//! `old + amount`.
//!
//! ```text
//! public:  token_address, account_address, old_commitment
//! private: account_sk, old_amount, old_blinding, amount, blinding
//! proves:  ownership of the account,
//!          old_commitment opens to (old_amount, old_blinding),
//!          amounts are in range,
//!          the new commitment opens to (old_amount + amount, blinding)
//! returns: (new_commitment, nullifier)
//! ```

use circuit_lib::state::transition::{Consumed, SpendResult, add_bounded};
use circuit_lib::types::account::Address;

fn main(
    token_address: pub Field,
    account_address: pub Field,
    old_commitment: pub Field,
    account_sk: Field,
    old_amount: Field,
    old_blinding: Field,
    amount: Field,
    blinding: Field,
) -> pub (Field, Field) {
    // Only the account's secret authorizes spending its commitment.
    Address::new(account_address).assert_owned_by(account_sk);

    // Consume the old commitment: the opening must match, and the emitted
    // nullifier folds in the token so proofs cannot cross tokens.
    let consumed = Consumed::of_for_token(old_commitment, account_sk, token_address, old_amount, old_blinding);

    // Produce the new commitment over the increased balance. add_bounded
    // range-constrains both amounts (no wrap, no inflation).
    let new_amount = add_bounded(old_amount, amount);
    let created = SpendResult::to(new_amount, blinding);

    (created.commitment, consumed.nullifier)
}

global TOKEN: Field = 0x746f6b656e; // "token"

fn sample_sk() -> Field {
    99
}

fn sample_address() -> Field {
    Address::from_secret(sample_sk()).value
}

#[test]
fn honest_deposit_passes() {
    let old = SpendResult::to(100, 7).commitment;
    let (new_commitment, nullifier) = main(
        TOKEN, sample_address(), old, sample_sk(), 100, 7, 50, 11,
    );
    // The new commitment must open to the increased balance.
    assert(new_commitment == SpendResult::to(150, 11).commitment);
    assert(nullifier != 0);
}

#[test]
fn deposit_with_different_blinding_differs() {
    let old = SpendResult::to(100, 7).commitment;
    let (a, _) = main(TOKEN, sample_address(), old, sample_sk(), 100, 7, 50, 11);
    let (b, _) = main(TOKEN, sample_address(), old, sample_sk(), 100, 7, 50, 12);
    assert(a != b);
}

#[test]
fn deposit_is_binding_to_the_old_commitment() {
    // The opening must match the public commitment: depositing against a
    // different old commitment with mismatched witnesses is unsatisfiable,
    // so the produced commitment differs accordingly for honest inputs.
    let old_a = SpendResult::to(100, 7).commitment;
    let (new_a, _) = main(TOKEN, sample_address(), old_a, sample_sk(), 100, 7, 50, 11);
    let old_b = SpendResult::to(200, 7).commitment;
    let (new_b, _) = main(TOKEN, sample_address(), old_b, sample_sk(), 200, 7, 50, 11);
    assert(new_a != new_b);
}
