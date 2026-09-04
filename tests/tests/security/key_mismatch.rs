//! Key mismatch: a proof verified under a different verification key must
//! fail, and verification under no matching key must not silently pass.

use crucible_interfaces::{VerificationFailure, VerificationRequest, Verifier};

/// A proof made under the mock key fails when the submitted verification key
/// id names a different key.
#[test]
fn wrong_verification_key_id_is_rejected() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut submission = VerificationRequest::from_response(&response);
    submission.verification_key_id =
        crucible_interfaces::VerificationKeyId::new("mock-vk/transfer/0.1.0-evil").unwrap();
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(outcome.rejected_with(VerificationFailure::WrongVerificationKey));
}

/// A verifier holding different key material (different digest key) rejects
/// the proof as invalid, not as a context mismatch.
#[test]
fn verifier_with_different_key_material_rejects() {
    let response = super::transfer_response();
    let foreign_verifier = crucible_mock::MockVerifier::with_key("attacker-key");
    let outcome = foreign_verifier
        .verify(&VerificationRequest::from_response(&response))
        .unwrap();
    assert!(outcome.rejected_with(VerificationFailure::InvalidProof));
}

/// Tampering the artifact checksum a proof claims must be caught: the proof
/// is then attributed to a different artifact than the one that made it.
#[test]
fn artifact_checksum_swap_is_rejected() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut submission = VerificationRequest::from_response(&response);
    submission.artifact_checksum =
        crucible_interfaces::ArtifactChecksum::from_bytes(b"a different artifact");
    let outcome = stack.verifier.verify(&submission).unwrap();
    assert!(outcome.rejected_with(VerificationFailure::ArtifactChecksumMismatch));
}

/// Control: with the correct key and checksum the proof verifies.
#[test]
fn matching_key_verifies() {
    let stack = super::stack();
    let response = super::transfer_response();
    assert!(stack.verify(&response).verified);
}
