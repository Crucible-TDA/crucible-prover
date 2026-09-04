//! Invariant: proofs are bound to their full context (state + identity). No
//! field of the context can change without invalidating the proof, and no
//! unbound proof can satisfy a state-bound operation.

use crucible_interfaces::{FieldValue, VerificationFailure, VerificationRequest, Verifier};
use crucible_mock::fixtures;

#[test]
fn state_binding_is_checked_for_state_bound_operations() {
    let stack = crucible_tests::MockStack::new();
    for request in fixtures::all_valid_requests() {
        // deposit/merge/transfer/withdraw touch existing state; register
        // creates an account and carries no binding in the fixture.
        let response = stack.prove(&request);
        let mut submission = VerificationRequest::from_response(&response);
        let needs_binding = matches!(
            request.operation,
            crucible_interfaces::Operation::Merge
                | crucible_interfaces::Operation::Transfer
                | crucible_interfaces::Operation::Withdraw
        );
        if needs_binding {
            submission.state_reference = None;
            let outcome = stack.verifier.verify(&submission).unwrap();
            assert!(
                outcome.rejected_with(VerificationFailure::MissingStateBinding),
                "{} without binding must be rejected: {outcome:?}",
                request.operation
            );
        }
    }
}

#[test]
fn every_context_field_mismatch_has_a_distinct_failure() {
    let stack = crucible_tests::MockStack::new();
    let response = stack.prove(&fixtures::transfer_request());

    type Case = (
        &'static str,
        Box<dyn Fn(&mut VerificationRequest)>,
        VerificationFailure,
    );
    let cases: Vec<Case> = vec![
        (
            "backend",
            Box::new(|v| v.backend = crucible_interfaces::BackendId::new("ultrahonk").unwrap()),
            VerificationFailure::BackendMismatch,
        ),
        (
            "circuit",
            Box::new(|v| v.circuit = crucible_interfaces::CircuitId::new("deposit").unwrap()),
            VerificationFailure::CircuitMismatch,
        ),
        (
            "version",
            Box::new(|v| v.circuit_version = crucible_interfaces::Version::new(2, 0, 0)),
            VerificationFailure::VersionMismatch,
        ),
        (
            "verification key",
            Box::new(|v| {
                v.verification_key_id =
                    crucible_interfaces::VerificationKeyId::new("other-vk").unwrap()
            }),
            VerificationFailure::WrongVerificationKey,
        ),
        (
            "artifact checksum",
            Box::new(|v| {
                v.artifact_checksum = crucible_interfaces::ArtifactChecksum::from_bytes(b"swapped")
            }),
            VerificationFailure::ArtifactChecksumMismatch,
        ),
        (
            "public outputs",
            Box::new(|v| {
                v.public_outputs
                    .insert("amount", FieldValue::from_hex("1234").unwrap())
                    .unwrap()
            }),
            VerificationFailure::PublicOutputMismatch,
        ),
    ];

    for (label, mutate, expected) in cases {
        let mut submission = VerificationRequest::from_response(&response);
        mutate(&mut submission);
        let outcome = stack.verifier.verify(&submission).unwrap();
        assert!(
            outcome.rejected_with(expected),
            "mismatched {label} gave {outcome:?}, expected {expected:?}"
        );
    }
}
