//! Invariant suite: properties that must hold for every operation, every
//! proof, and every backend — the contract-level guarantees of Crucible.

#[path = "invariants/context_binding.rs"]
mod context_binding;
#[path = "invariants/proof_verifies.rs"]
mod proof_verifies;
#[path = "invariants/public_inputs_bound.rs"]
mod public_inputs_bound;
#[path = "invariants/witness_not_public.rs"]
mod witness_not_public;

use crucible_interfaces::{VerificationRequest, Verifier};
use crucible_mock::fixtures;
use crucible_tests::MockStack;

/// Every valid fixture, when proved through the composed service, must
/// produce a proof that verifies against its own response.
#[test]
fn every_valid_request_produces_a_verifying_proof() {
    let stack = MockStack::new();
    for request in fixtures::all_valid_requests() {
        let response = stack.prove(&request);
        let outcome = stack
            .verifier
            .verify(&VerificationRequest::from_response(&response))
            .expect("verifier runs");
        assert!(outcome.verified, "{} failed: {outcome}", request.operation);
        // The response must echo the request's identity exactly.
        assert_eq!(response.circuit, request.circuit);
        assert_eq!(response.circuit_version, request.circuit_version);
        assert_eq!(response.backend, request.backend);
    }
}

/// Every valid proof must round-trip through the envelope JSON unchanged.
#[test]
fn every_proof_survives_envelope_round_trip() {
    let stack = MockStack::new();
    for request in fixtures::all_valid_requests() {
        let response = stack.prove(&request);
        let envelope = crucible_proof_types::ProofEnvelope::from_response(
            &response,
            request.operation,
            "invariants",
        );
        let json = envelope.to_json().unwrap();
        let back = crucible_proof_types::ProofEnvelope::from_json(&json).unwrap();
        assert_eq!(
            back, envelope,
            "{} envelope did not round-trip",
            request.operation
        );
    }
}
