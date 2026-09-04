//! Measurement gadget: one full confidential state transition.
//!
//! The atomic unit every operation circuit performs: consume an old
//! commitment (opening + token-bound nullifier) and produce one new
//! commitment. This is the floor cost of deposit/merge/transfer/withdraw
//! before their operation-specific arithmetic.

use circuit_lib::crypto::commit;
use circuit_lib::state::transition::{Consumed, SpendResult};

fn main(
    old_commitment: pub Field,
    token: pub Field,
    account_sk: Field,
    old_amount: Field,
    old_blinding: Field,
    new_amount: Field,
    new_blinding: Field,
) -> pub (Field, Field) {
    let spent = Consumed::of_for_token(
        old_commitment,
        account_sk,
        token,
        old_amount,
        old_blinding,
    );
    let produced = SpendResult::to(new_amount, new_blinding);
    (produced.commitment, spent.nullifier)
}

#[test]
fn gadget_consumes_and_produces() {
    let old = commit(100, 5);
    let (new_c, nullifier) = main(old, 0x746f6b656e, 0x1234, 100, 5, 140, 9);
    assert(new_c == commit(140, 9));
    assert(nullifier != 0);
    // The produced commitment differs from the consumed one.
    assert(new_c != old);
}
