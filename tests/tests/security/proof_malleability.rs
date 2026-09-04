//! Proof malleability: any change to the proof bytes must invalidate it.

use crucible_interfaces::{VerificationFailure, VerificationRequest, Verifier};

/// Flips bytes at several positions of a proof and asserts rejection every
/// time. Positions include the header, the middle of the payload, and the
/// trailing digest.
#[test]
fn every_byte_flip_invalidates_the_proof() {
    let stack = super::stack();
    let response = super::transfer_response();
    let positions = [
        0usize,
        5,
        40,
        response.proof.bytes.len() / 2,
        response.proof.bytes.len() - 1,
    ];

    for position in positions {
        let mut tampered = response.clone();
        tampered.proof.bytes[position] ^= 0x01;
        let outcome = stack.verify(&tampered);
        assert!(
            outcome.rejected_with(VerificationFailure::InvalidProof),
            "byte flip at {position} was not rejected as InvalidProof: {outcome:?}"
        );
    }
}

/// Appending garbage to a proof must invalidate it (the trailing digest is
/// then in the wrong place and the check fails).
#[test]
fn appended_bytes_invalidate_the_proof() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut tampered = response.clone();
    tampered.proof.bytes.extend_from_slice(b"garbage");
    let outcome = stack.verify(&tampered);
    assert!(!outcome.verified, "appended bytes must be rejected");
}

/// Truncating the proof must invalidate it.
#[test]
fn truncated_proof_is_rejected() {
    let stack = super::stack();
    let response = super::transfer_response();
    let mut tampered = response.clone();
    tampered
        .proof
        .bytes
        .truncate(response.proof.bytes.len() - 10);
    let outcome = stack.verify(&tampered);
    assert!(!outcome.verified, "truncation must be rejected");
}

/// A proof forged by mixing two valid proofs' halves must not verify.
#[test]
fn spliced_proof_halves_do_not_verify() {
    let stack = super::stack();
    let response = super::transfer_response();
    let other = {
        let stack = super::stack();
        stack.prove(&crucible_mock::fixtures::withdraw_request())
    };
    // Splice the first half of the other proof over the first half of this
    // one. Both halves come from index 0 so the copied slice length matches
    // regardless of total proof lengths.
    let mut spliced = response.clone();
    let mid = response.proof.bytes.len() / 2;
    spliced.proof.bytes[..mid].copy_from_slice(&other.proof.bytes[..mid]);
    assert_ne!(
        spliced.proof.bytes, response.proof.bytes,
        "splice changed nothing"
    );
    let outcome = stack.verify(&spliced);
    assert!(!outcome.verified, "spliced proof must be rejected");
}

/// Sanity check used across the suite: the untouched proof still verifies
/// through the full request path.
#[test]
fn untouched_proof_still_verifies() {
    let stack = super::stack();
    let response = super::transfer_response();
    let verification = VerificationRequest::from_response(&response);
    assert!(stack.verifier.verify(&verification).unwrap().verified);
}
