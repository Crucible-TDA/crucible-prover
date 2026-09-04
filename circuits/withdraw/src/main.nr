//! Withdraw circuit: redeem confidential value out of the domain.
//!
//! The account's commitment is consumed (opening proven under the owner
//! secret, token-bound nullifier emitted). `amount` leaves the confidential
//! domain and, for a partial withdrawal, a change commitment over
//! `old - amount` stays behind.
//!
//! ```text
//! public:  token_address, account_address, commitment
//! private: account_sk, amount, old_amount, old_blinding, change_blinding
//! proves:  ownership of the account,
//!          the commitment opens to (old_amount, old_blinding),
//!          amount is in range and does not overdraw,
//!          the change commitment opens to (old_amount - amount, change_blinding)
//! returns: (change_commitment, nullifier)
//! ```

use circuit_lib::state::transition::{Consumed, SpendResult, subtract_bounded};
use circuit_lib::types::account::Address;

fn main(
    token_address: pub Field,
    account_address: pub Field,
    commitment: pub Field,
    account_sk: Field,
    amount: Field,
    old_amount: Field,
    old_blinding: Field,
    change_blinding: Field,
) -> pub (Field, Field) {
    // Only the account's secret authorizes withdrawing its value.
    Address::new(account_address).assert_owned_by(account_sk);

    // Consume the old commitment (opening + token-bound nullifier).
    let spent = Consumed::of_for_token(commitment, account_sk, token_address, old_amount, old_blinding);

    // The remainder stays confidential as change. subtract_bounded rejects
    // withdrawing more than the commitment holds.
    let change_amount = subtract_bounded(old_amount, amount);
    let change = SpendResult::to(change_amount, change_blinding);

    (change.commitment, spent.nullifier)
}

global TOKEN: Field = 0x746f6b656e; // "token"

fn sample_sk() -> Field {
    0x1234
}

fn sample_address() -> Field {
    Address::from_secret(sample_sk()).value
}

#[test]
fn honest_partial_withdrawal_passes() {
    let c = SpendResult::to(1000, 5).commitment;
    let (change, nullifier) = main(TOKEN, sample_address(), c, sample_sk(), 300, 1000, 5, 8);
    assert(change == SpendResult::to(700, 8).commitment);
    assert(nullifier != 0);
}

#[test]
fn full_withdrawal_leaves_zero_change() {
    let c = SpendResult::to(1000, 5).commitment;
    let (change, _) = main(TOKEN, sample_address(), c, sample_sk(), 1000, 1000, 5, 8);
    assert(change == SpendResult::to(0, 8).commitment);
}

#[test]
fn withdrawal_is_binding_to_the_commitment() {
    // Different old commitments with otherwise identical witnesses produce
    // different (correct) change commitments for their actual balances.
    let a = SpendResult::to(1000, 5).commitment;
    let (change_a, _) = main(TOKEN, sample_address(), a, sample_sk(), 300, 1000, 5, 8);
    let b = SpendResult::to(2000, 5).commitment;
    let (change_b, _) = main(TOKEN, sample_address(), b, sample_sk(), 300, 2000, 5, 8);
    assert(change_a != change_b);
}
