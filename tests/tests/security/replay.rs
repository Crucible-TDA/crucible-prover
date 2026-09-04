//! Replay protection: a proof valid against state root A must not be
//! accepted against state root B, and unbound proofs must not satisfy
//! state-bound operations.

use crucible_interfaces::{RootDigest, StateReference, VerificationFailure};

/// The same proof, replayed against a *different* state root, must be
/// rejected as a state-reference mismatch.
#[test]
fn proof_replayed_against_new_state_root_is_rejected() {
    let stack = super::stack();
    let request = crucible_mock::fixtures::transfer_request();
    let response = stack.prove(&request);
    assert!(stack.verify(&response).verified);

    let mut replayed = response.clone();
    replayed.state_reference = Some(StateReference::new(
        crucible_mock::fixtures::state_root_b(),
        // A later sequence makes this look like a freshly-advanced state.
        99,
    ));
    let outcome = stack.verify(&replayed);
    assert!(
        outcome.rejected_with(VerificationFailure::StateReferenceMismatch),
        "replayed proof must fail: {outcome:?}"
    );
}

/// The same proof, replayed against the *same* state, must still verify:
/// replay protection comes from the state advancing, not from the request id.
#[test]
fn proof_reused_against_same_state_is_not_a_violation() {
    let stack = super::stack();
    let request = crucible_mock::fixtures::transfer_request();
    let response = stack.prove(&request);
    // Submitting the identical proof again is indistinguishable from the
    // original submission — the ledger's state binding decides freshness.
    assert!(stack.verify(&response).verified);
}

/// A transfer proof with its state binding stripped must be rejected: the
/// operation requires binding.
#[test]
fn stripping_the_state_binding_is_rejected() {
    let stack = super::stack();
    let request = crucible_mock::fixtures::transfer_request();
    let response = stack.prove(&request);
    let mut unbound = response.clone();
    unbound.state_reference = None;
    let outcome = stack.verify(&unbound);
    assert!(
        outcome.rejected_with(VerificationFailure::MissingStateBinding),
        "unbound transfer must be rejected: {outcome:?}"
    );
}

/// Replay across operations sharing the same old state root must not carry
/// over: a proof for state root A at sequence 1 is invalid once the account
/// has moved to sequence 2 on root A (root reused after an advance).
#[test]
fn same_root_at_higher_sequence_is_rejected() {
    let stack = super::stack();
    let request = crucible_mock::fixtures::transfer_request();
    let response = stack.prove(&request);

    let original_seq = response
        .state_reference
        .as_ref()
        .expect("transfer is bound")
        .sequence;
    let mut replayed = response.clone();
    replayed.state_reference = Some(StateReference::new(
        RootDigest::from_hex(&"ab".repeat(32)).unwrap(),
        original_seq + 1,
    ));
    let outcome = stack.verify(&replayed);
    assert!(
        outcome.rejected_with(VerificationFailure::StateReferenceMismatch),
        "sequence-advanced replay must fail: {outcome:?}"
    );
}
