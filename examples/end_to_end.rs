//! End-to-end lifecycle demonstration: register → deposit → merge →
//! transfer → withdraw through the mock backend.
//!
//! Each step loads the operation's committed valid vector, proves it
//! through [`ProverService`], and verifies the response locally — the same
//! seams the CLI and the integration suites use. The mock backend is TEST
//! ONLY (no cryptography); run with the CLI and `--backend ultrahonk` for
//! real proofs.

use std::path::Path;

use crucible_interfaces::Verifier;
use crucible_interfaces::prover::Prover;
use crucible_mock::{MockProver, MockVerifier};
use crucible_proof_types::ProofEnvelope;
use crucible_prover_core::ProverService;
use crucible_vectors::TestVector;

/// The five operation circuits in lifecycle order.
const LIFECYCLE: [&str; 5] = ["register", "deposit", "merge", "transfer", "withdraw"];

fn catalog_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors")
}

fn main() {
    let mut service = ProverService::new("crucible-examples/0.1.0");
    service.register_provider(Box::new(MockProver::new()));
    let verifier = MockVerifier::new();
    service.with_local_verifier(Box::new(verifier.clone()));

    println!("end-to-end (mock backend — TEST ONLY, not cryptographically secure)");
    println!();
    for op in LIFECYCLE {
        let path = catalog_root()
            .join(op)
            .join("valid")
            .join(format!("{op}-valid-001.json"));
        let vector = TestVector::load(&path)
            .unwrap_or_else(|e| panic!("cannot load vector `{}`: {e}", path.display()));
        let request = vector.to_request();
        request.validate().expect("vector forms a valid request");

        // Prove and require the local round trip (prove_and_verify refuses
        // to return a proof that fails its own verification).
        let response = service
            .prove_and_verify(&request)
            .expect("proof must pass its own round trip")
            .0;
        let outcome = verifier
            .verify(&crucible_interfaces::VerificationRequest::from_response(
                &response,
            ))
            .expect("verifier runs");
        assert!(outcome.verified, "proof must verify");

        let envelope =
            ProofEnvelope::from_response(&response, vector.operation, "crucible-examples");
        let state = response
            .state_reference
            .as_ref()
            .map(|s| format!("root {}", s.root))
            .unwrap_or_else(|| "unbound".to_owned());
        println!(
            "{:<10} verified — {} public word(s), {} proof bytes, {}",
            op,
            response.public_outputs.len(),
            response.proof.bytes.len(),
            state
        );
        // The envelope JSON round-trips losslessly (serialization demo).
        let json = envelope.to_json().expect("envelope serializes");
        let back = ProofEnvelope::from_json(&json).expect("envelope parses");
        assert_eq!(back, envelope);
    }
    println!();
    println!("lifecycle complete: all five operations proved and verified");
}
