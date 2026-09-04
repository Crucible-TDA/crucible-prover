//! Wrong-context protection: a valid proof submitted under a different
//! circuit, version, or public-output set must be rejected with the specific
//! reason, never silently accepted.

use crucible_interfaces::{
    CircuitId, FieldValue, Operation, VerificationFailure, VerificationRequest, Verifier,
};

/// A proof for the transfer circuit submitted as a withdraw proof fails.
#[test]
fn circuit_swap_is_rejected() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut submission = VerificationRequest::from_response(&response);
    submission.circuit = CircuitId::for_operation(Operation::Withdraw);
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(outcome.rejected_with(VerificationFailure::CircuitMismatch));
}

/// A proof for circuit version 0.1.0 submitted as 1.0.0 fails.
#[test]
fn version_swap_is_rejected() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut submission = VerificationRequest::from_response(&response);
    submission.circuit_version = crucible_interfaces::Version::new(1, 0, 0);
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(outcome.rejected_with(VerificationFailure::VersionMismatch));
}

/// Altering a public output after the proof was made fails with
/// PublicOutputMismatch — this is the stale-public-input protection.
#[test]
fn altered_public_outputs_are_rejected() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut submission = VerificationRequest::from_response(&response);
    // Rewrite one existing output to a different value.
    let first_name = response
        .public_outputs
        .names()
        .next()
        .expect("fixture has outputs")
        .to_owned();
    submission
        .public_outputs
        .set(&first_name, FieldValue::from_hex("deadbeef").unwrap())
        .unwrap();
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(outcome.rejected_with(VerificationFailure::PublicOutputMismatch));
}

/// Submitting under a different backend label fails cleanly.
#[test]
fn backend_swap_is_rejected() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut submission = VerificationRequest::from_response(&response);
    submission.backend = crucible_interfaces::BackendId::new("ultrahonk").unwrap();
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(outcome.rejected_with(VerificationFailure::BackendMismatch));
}

/// The control case: the same response under its own context verifies.
#[test]
fn correct_context_verifies() {
    let stack = super::stack();
    let response = super::transfer_response();
    let outcome = stack
        .verifier
        .verify(&VerificationRequest::from_response(&response))
        .unwrap();
    assert!(outcome.verified);
}
