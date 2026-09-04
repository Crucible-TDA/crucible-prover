//! Merge circuit: consolidate two of an account's commitments into one.
//!
//! Accounts accumulate dust commitments; merging reduces the commitment
//! count. The circuit consumes both old commitments (proving each opening
//! under the owner secret, emitting two nullifiers) and returns a single
//! commitment over their sum.
//!
//! Merge is a **state-bound** operation: the two halves of the ledger state
//! root the merge runs against are public inputs folded into the nullifiers,
//! so a proof cut for root A cannot be replayed against root B (see
//! `docs/ultrahonk.md`).
//!
//! ```text
//! public:  token_address, account_address, commitment_a, commitment_b,
//!          root_hi, root_lo
//! private: account_sk, amount_a, blinding_a, amount_b, blinding_b, blinding
//! proves:  ownership of the account,
//!          both commitments open to their supplied witnesses,
//!          amounts are in range,
//!          the merged commitment opens to (amount_a + amount_b, blinding),
//!          the nullifiers are scoped to (token, state root)
//! returns: (merged_commitment, nullifier_a, nullifier_b)
//! ```

use circuit_lib::state::transition::{Consumed, SpendResult, add_bounded};
use circuit_lib::types::account::Address;

fn main(
    token_address: pub Field,
    account_address: pub Field,
    commitment_a: pub Field,
    commitment_b: pub Field,
    root_hi: pub Field,
    root_lo: pub Field,
    account_sk: Field,
    amount_a: Field,
    blinding_a: Field,
    amount_b: Field,
    blinding_b: Field,
    blinding: Field,
) -> pub (Field, Field, Field) {
    // Only the account's secret authorizes spending its commitments.
    Address::new(account_address).assert_owned_by(account_sk);

    // Consume both commitments under the same owner, token, and state root.
    let spent_a = Consumed::of_for_token_state(
        commitment_a, account_sk, token_address, root_hi, root_lo, amount_a, blinding_a,
    );
    let spent_b = Consumed::of_for_token_state(
        commitment_b, account_sk, token_address, root_hi, root_lo, amount_b, blinding_b,
    );

    // The merged commitment covers the sum; add_bounded prevents wrap.
    let merged_amount = add_bounded(amount_a, amount_b);
    let merged = SpendResult::to(merged_amount, blinding);

    (merged.commitment, spent_a.nullifier, spent_b.nullifier)
}

global TOKEN: Field = 0x746f6b656e; // "token"

fn sample_sk() -> Field {
    77
}

fn sample_address() -> Field {
    Address::from_secret(sample_sk()).value
}

fn root_hi() -> Field {
    0xababababababababababababababab
}

fn root_lo() -> Field {
    0xababababababababababababababab
}

fn other_root() -> Field {
    0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd
}

#[test]
fn honest_merge_passes() {
    let a = SpendResult::to(60, 1).commitment;
    let b = SpendResult::to(40, 2).commitment;
    let (merged, nullifier_a, nullifier_b) = main(
        TOKEN, sample_address(), a, b, root_hi(), root_lo(), sample_sk(), 60, 1, 40, 2, 9,
    );
    assert(merged == SpendResult::to(100, 9).commitment);
    assert(nullifier_a != 0);
    assert(nullifier_b != 0);
    // Two distinct commitments yield two distinct nullifiers.
    assert(nullifier_a != nullifier_b);
}

#[test]
fn merge_sums_in_any_input_order() {
    let a = SpendResult::to(60, 1).commitment;
    let b = SpendResult::to(40, 2).commitment;
    let (forward, _, _) = main(
        TOKEN, sample_address(), a, b, root_hi(), root_lo(), sample_sk(), 60, 1, 40, 2, 9,
    );
    let (reversed, _, _) = main(
        TOKEN, sample_address(), b, a, root_hi(), root_lo(), sample_sk(), 40, 2, 60, 1, 9,
    );
    assert(forward == reversed);
}

#[test]
fn merged_amount_is_exact() {
    let a = SpendResult::to(1, 1).commitment;
    let b = SpendResult::to(1, 2).commitment;
    let (merged, _, _) =
        main(TOKEN, sample_address(), a, b, root_hi(), root_lo(), sample_sk(), 1, 1, 1, 2, 3);
    assert(merged == SpendResult::to(2, 3).commitment);
}

#[test]
fn state_bound_nullifiers_differ_across_roots() {
    let a = SpendResult::to(60, 1).commitment;
    let b = SpendResult::to(40, 2).commitment;
    let (_, n_a1, n_b1) = main(
        TOKEN, sample_address(), a, b, root_hi(), root_lo(), sample_sk(), 60, 1, 40, 2, 9,
    );
    let (_, n_a2, n_b2) = main(
        TOKEN, sample_address(), a, b, other_root(), other_root(), sample_sk(), 60, 1, 40, 2, 9,
    );
    assert(n_a1 != n_a2);
    assert(n_b1 != n_b2);
}
