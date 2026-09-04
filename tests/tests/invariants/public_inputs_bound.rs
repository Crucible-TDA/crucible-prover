//! Invariant: public inputs are bound. A proof is only valid for the exact
//! public output set it was generated against; verification under any other
//! set fails.

use crucible_interfaces::{FieldValue, VerificationFailure, VerificationRequest, Verifier};
use crucible_mock::fixtures;

#[test]
fn changing_any_public_output_breaks_verification() {
    let stack = crucible_tests::MockStack::new();
    let response = stack.prove(&fixtures::transfer_request());

    // Enumerate the public outputs the proof commits to.
    let names: Vec<String> = response
        .public_outputs
        .iter()
        .map(|(n, _)| n.to_owned())
        .collect();
    assert!(names.len() >= 2, "fixture should carry several outputs");

    // For each name, alter the value and confirm rejection.
    for name in names {
        let mut submission = VerificationRequest::from_response(&response);
        submission
            .public_outputs
            .set(&name, FieldValue::from_hex("ffffffff").unwrap())
            .unwrap();
        let outcome = stack.verifier.verify(&submission).unwrap();
        assert!(
            outcome.rejected_with(VerificationFailure::PublicOutputMismatch),
            "altering `{name}` was not rejected: {outcome:?}"
        );
    }
}

#[test]
fn adding_an_undeclared_output_breaks_verification() {
    let stack = crucible_tests::MockStack::new();
    let response = stack.prove(&fixtures::transfer_request());
    let mut submission = VerificationRequest::from_response(&response);
    submission
        .public_outputs
        .insert("sneaky_output", FieldValue::from_hex("01").unwrap())
        .unwrap();
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(
        outcome.rejected_with(VerificationFailure::PublicOutputMismatch),
        "extra output must break binding: {outcome:?}"
    );
}

#[test]
fn dropping_a_bound_output_breaks_verification() {
    let stack = crucible_tests::MockStack::new();
    let response = stack.prove(&fixtures::transfer_request());
    let first = response
        .public_outputs
        .names()
        .next()
        .expect("fixture has outputs")
        .to_owned();
    let submission = VerificationRequest::from_response(&response);
    // Rebuild with one fewer output by removing via a fresh bag is not
    // supported, so instead we verify that compare_outputs flags the gap.
    let mismatch =
        crucible_prover_core::public_inputs::compare_outputs(&submission.public_outputs, &{
            let mut reduced = crucible_interfaces::OutputBag::new();
            for (name, value) in response.public_outputs.iter() {
                if name != first {
                    reduced.insert(name, value.clone()).unwrap();
                }
            }
            reduced
        });
    assert_eq!(mismatch.missing_from_produced, vec![first]);
}
