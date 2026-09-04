//! Invariant: a freshly generated proof always verifies (against its own
//! response) for every operation and every backend registered.

use crucible_interfaces::{VerificationRequest, Verifier};
use crucible_mock::fixtures;
use crucible_tests::MockStack;

#[test]
fn mock_proofs_verify_for_all_five_operations() {
    let stack = MockStack::new();
    for request in fixtures::all_valid_requests() {
        let response = stack.prove(&request);
        let outcome = stack
            .verifier
            .verify(&VerificationRequest::from_response(&response))
            .unwrap();
        assert!(
            outcome.verified,
            "operation {} did not verify: {outcome}",
            request.operation
        );
    }
}

#[test]
fn proof_generation_is_repeatable_and_stable() {
    let stack = MockStack::new();
    let request = fixtures::transfer_request();
    let a = stack.prove(&request);
    let b = stack.prove(&request);
    // Deterministic mock backend: same request, same proof bytes.
    assert_eq!(a.proof.bytes, b.proof.bytes);
    assert_eq!(a, b);
}
