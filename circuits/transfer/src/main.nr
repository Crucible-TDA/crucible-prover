//! Transfer circuit: move confidential value from a sender commitment to a
//! recipient.
//!
//! The sender's old commitment is consumed (opening proven under the sender
//! secret, token-bound nullifier emitted) and split into two new
//! commitments: one paying `amount` to the recipient, one returning the
//! change to the sender. Sender and recipient addresses stay public — this
//! is exactly the Confidential Token privacy shape, where who moves value is
//! visible but how much is not.
//!
//! ```text
//! public:  token_address, sender_address, recipient_address, old_sender_commitment
//! private: sender_sk, amount, old_amount, old_blinding,
//!          recipient_blinding, change_blinding
//! proves:  sender owns the consumed commitment,
//!          the old commitment opens to (old_amount, old_blinding),
//!          amount is in range and does not overdraw,
//!          recipient commitment opens to (amount, recipient_blinding),
//!          change commitment opens to (old_amount - amount, change_blinding)
//! returns: (recipient_commitment, change_commitment, nullifier)
//! ```

use circuit_lib::state::transition::{Consumed, SpendResult, subtract_bounded};
use circuit_lib::types::account::Address;

fn main(
    token_address: pub Field,
    sender_address: pub Field,
    recipient_address: pub Field,
    old_sender_commitment: pub Field,
    sender_sk: Field,
    amount: Field,
    old_amount: Field,
    old_blinding: Field,
    recipient_blinding: Field,
    change_blinding: Field,
) -> pub (Field, Field, Field) {
    // Only the sender's secret can spend the sender's commitment.
    Address::new(sender_address).assert_owned_by(sender_sk);

    // Consume the sender's old commitment (opening + token-bound nullifier).
    let spent = Consumed::of_for_token(
        old_sender_commitment, sender_sk, token_address, old_amount, old_blinding,
    );

    // Split the balance: the recipient gets `amount`, the sender keeps the
    // change. subtract_bounded range-constrains and rejects overdraw.
    let change_amount = subtract_bounded(old_amount, amount);

    let recipient = SpendResult::to(amount, recipient_blinding);
    let change = SpendResult::to(change_amount, change_blinding);

    // `recipient_address` is public context (addresses are visible in the
    // Confidential Token model); the ledger attaches the returned recipient
    // commitment to that address. The opening is transmitted off-chain, so
    // no in-circuit recipient binding is needed in this scaffold — see
    // docs/noir.md for the encryption/key-binding alignment.
    let _ = recipient_address;
    (recipient.commitment, change.commitment, spent.nullifier)
}

global TOKEN: Field = 0x746f6b656e; // "token"

fn sender_sk() -> Field {
    0xaaaa
}

fn sender_address() -> Field {
    Address::from_secret(sender_sk()).value
}

fn recipient_address() -> Field {
    0xbbbb
}

#[test]
fn honest_transfer_passes() {
    let old = SpendResult::to(1000, 3).commitment;
    let (recipient, change, nullifier) = main(
        TOKEN, sender_address(), recipient_address(), old,
        sender_sk(), 400, 1000, 3, 21, 22,
    );
    // Recipient commitment opens to the transfer amount.
    assert(recipient == SpendResult::to(400, 21).commitment);
    // Change commitment opens to the remainder.
    assert(change == SpendResult::to(600, 22).commitment);
    assert(nullifier != 0);
}

#[test]
fn full_balance_transfer_leaves_zero_change() {
    let old = SpendResult::to(400, 3).commitment;
    let (recipient, change, _) = main(
        TOKEN, sender_address(), recipient_address(), old,
        sender_sk(), 400, 400, 3, 21, 22,
    );
    assert(recipient == SpendResult::to(400, 21).commitment);
    assert(change == SpendResult::to(0, 22).commitment);
}

#[test]
fn blinding_changes_the_commitments_not_the_amounts() {
    let old = SpendResult::to(1000, 3).commitment;
    let (a_recipient, a_change, _) = main(
        TOKEN, sender_address(), recipient_address(), old,
        sender_sk(), 400, 1000, 3, 21, 22,
    );
    let (b_recipient, b_change, _) = main(
        TOKEN, sender_address(), recipient_address(), old,
        sender_sk(), 400, 1000, 3, 99, 98,
    );
    // Different blinding => different commitment bytes, same underlying
    // amounts (both must still open correctly by construction).
    assert(a_recipient != b_recipient);
    assert(a_change != b_change);
}
