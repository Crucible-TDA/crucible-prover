//! Stale-state protection: a proof generated against an old snapshot of an
//! account's state must be rejected once the state has moved on.

use crucible_interfaces::{
    RootDigest, StateReference, VerificationFailure, VerificationRequest, Verifier,
};

/// Simulates the scenario from the design docs: Alice generates a proof,
/// performs another operation (state advances), then the old proof is
/// submitted. The old proof binds to the pre-advance root and must fail.
#[test]
fn proof_from_before_a_state_advance_is_rejected() {
    let stack = super::stack();
    let request = crucible_mock::fixtures::transfer_request();
    let stale_proof = stack.prove(&request);

    // The state has since advanced: the root changed and the sequence moved.
    let advanced = StateReference::new(crucible_mock::fixtures::state_root_b(), 2);
    let mut submission = VerificationRequest::from_response(&stale_proof);
    submission.state_reference = Some(advanced.clone());

    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(
        outcome.rejected_with(VerificationFailure::StateReferenceMismatch),
        "stale proof must be rejected: {outcome:?}"
    );
}

/// The mock account state used by deposit/transfer scenarios keeps the same
/// root for a while: a proof bound to the current root verifies while the
/// root is current.
#[test]
fn proof_against_current_state_verifies() {
    let stack = super::stack();
    let request = crucible_mock::fixtures::transfer_request();
    let proof = stack.prove(&request);
    assert!(stack.verify(&proof).verified);
}

/// Sequence is part of the binding: two snapshots with the same root but
/// different sequences are different states.
#[test]
fn sequence_advance_on_same_root_is_a_new_state() {
    let stack = super::stack();
    let request = crucible_mock::fixtures::merge_request();
    let proof = stack.prove(&request);

    let seq = proof
        .state_reference
        .as_ref()
        .expect("merge is state-bound")
        .sequence;
    let mut submission = VerificationRequest::from_response(&proof);
    submission.state_reference = Some(StateReference::new(
        RootDigest::from_hex(&"ab".repeat(32)).unwrap(),
        seq + 1,
    ));
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(
        !outcome.verified,
        "sequence advance must invalidate the proof"
    );
}
