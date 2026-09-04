//! Verification service tests: dispatch through crucible-verifier, envelope
//! round-trips, and wire-level corruption.

use crucible_interfaces::VerificationRequest;
use crucible_mock::fixtures;
use crucible_tests::MockStack;

/// A fresh proof verified through the composed stack passes.
#[test]
fn composed_stack_prove_verify_round_trip() {
    let stack = MockStack::new();
    for request in fixtures::all_valid_requests() {
        let response = stack.prove(&request);
        assert!(stack.verify(&response).verified);
    }
}

/// The VerificationService dispatches to every registered verifier; with two
/// matching verifiers the report must be unanimous.
#[test]
fn verification_service_reports_unanimous_acceptance() {
    let stack = MockStack::new();
    let response = stack.prove(&fixtures::transfer_request());

    let mut service = crucible_verifier::VerificationService::new();
    service
        .register_verifier("local", Box::new(crucible_mock::MockVerifier::new()))
        .unwrap();
    service
        .register_verifier("second", Box::new(crucible_mock::MockVerifier::new()))
        .unwrap();
    let report = service
        .verify(&VerificationRequest::from_response(&response))
        .unwrap();
    assert!(report.all_verified());
    assert!(!report.disagrees());
}

/// When one verifier disagrees (different key material), the report must
/// surface the disagreement rather than picking a winner.
#[test]
fn verification_service_surfaces_disagreement() {
    let stack = MockStack::new();
    let response = stack.prove(&fixtures::transfer_request());

    let mut service = crucible_verifier::VerificationService::new();
    service
        .register_verifier("local", Box::new(crucible_mock::MockVerifier::new()))
        .unwrap();
    service
        .register_verifier(
            "onchain-sim",
            Box::new(crucible_mock::MockVerifier::with_key("other")),
        )
        .unwrap();
    let report = service
        .verify(&VerificationRequest::from_response(&response))
        .unwrap();
    assert!(report.disagrees(), "a split verdict must be reported");
    assert_eq!(report.verified_by(), vec!["local"]);
}

/// Envelope JSON survives a full serialization round-trip and rejects a
/// future envelope version.
#[test]
fn envelope_version_gate_rejects_future_versions() {
    let stack = MockStack::new();
    let request = fixtures::transfer_request();
    let response = stack.prove(&request);
    let envelope = crucible_proof_types::ProofEnvelope::from_response(
        &response,
        request.operation,
        "verification-tests",
    );
    let json = envelope.to_json().unwrap();
    let tampered = json.replacen("\"version\":1", "\"version\":42", 1);
    assert!(matches!(
        crucible_proof_types::ProofEnvelope::from_json(&tampered),
        Err(crucible_proof_types::EnvelopeError::UnsupportedVersion { .. })
    ));
}

/// Malformed wire JSON is rejected, never guessed at.
#[test]
fn malformed_envelope_json_is_rejected() {
    assert!(crucible_proof_types::ProofEnvelope::from_json("{not json").is_err());
    assert!(crucible_proof_types::ProofEnvelope::from_json("").is_err());
}

/// Envelope serialization is deterministic for identical responses.
#[test]
fn envelope_serialization_is_deterministic() {
    let stack = MockStack::new();
    let request = fixtures::transfer_request();
    let a = stack.prove(&request);
    let b = stack.prove(&request);
    let ea = crucible_proof_types::ProofEnvelope::from_response(&a, request.operation, "t");
    let eb = crucible_proof_types::ProofEnvelope::from_response(&b, request.operation, "t");
    assert_eq!(ea.to_json().unwrap(), eb.to_json().unwrap());
}
