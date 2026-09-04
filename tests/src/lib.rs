//! Shared helpers for the cross-crate test suite.
//!
//! The integration tests under `tests/tests/` each compile as their own
//! binary; this lib hosts helpers they all link against so setup code is not
//! duplicated per binary.
#![forbid(unsafe_code)]

/// Circuit-tier execution against real Noir packages (nargo-gated).
pub mod circuit_runner;
/// Loader and structural validation for the `test-vectors/` JSON catalog.
pub mod vectors;

use crucible_interfaces::prover::Prover;
use crucible_interfaces::{ProofResponse, VerificationRequest, Verifier};
use crucible_mock::{MockProver, MockVerifier};
use crucible_prover_core::ProverService;

/// A fully wired mock stack: prover service + matching verifier.
pub struct MockStack {
    /// The proving facade.
    pub service: ProverService,
    /// The matching local verifier.
    pub verifier: MockVerifier,
}

impl MockStack {
    /// Builds a fresh stack with a default key pair.
    pub fn new() -> MockStack {
        let mut service = ProverService::new("crucible-tests/0.1.0");
        service.register_provider(Box::new(MockProver::new()));
        service.with_local_verifier(Box::new(MockVerifier::new()));
        MockStack {
            service,
            verifier: MockVerifier::new(),
        }
    }

    /// Generates a proof for `request` via the service and verifies it
    /// against its own response, returning the response.
    pub fn prove(&self, request: &crucible_interfaces::ProofRequest) -> ProofResponse {
        self.service
            .prove_and_verify(request)
            .expect("proof must pass its own round-trip")
            .0
    }

    /// Verifies a response (or a mutated copy) with the local verifier.
    pub fn verify(&self, response: &ProofResponse) -> crucible_interfaces::VerificationOutcome {
        self.verifier
            .verify(&VerificationRequest::from_response(response))
            .expect("verifier runs")
    }
}

impl Default for MockStack {
    fn default() -> MockStack {
        MockStack::new()
    }
}
