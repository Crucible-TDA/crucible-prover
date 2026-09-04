//! Mock round-trip bench: prove → verify → envelope for each operation.
//!
//! Exercises the full orchestration path (request validation, provider
//! dispatch, envelope assembly, verification) at mock speed, so it measures
//! Crucible's own overhead — never cryptographic cost.

use crucible_benches::{best_ns, fmt_ns, valid_vectors};
use crucible_interfaces::prover::Prover;
use crucible_interfaces::{VerificationRequest, Verifier};
use crucible_mock::{MockProver, MockVerifier};
use crucible_proof_types::ProofEnvelope;
use crucible_prover_core::ProverService;

fn main() {
    const SAMPLES: usize = 200;
    println!("mock round trip (prove + verify + envelope json), best of {SAMPLES}");
    println!("{:<28} ns/op", "vector");

    let vectors = valid_vectors();
    for vector in &vectors {
        let mut service = ProverService::new("crucible-benches/0.1.0");
        service.register_provider(Box::new(MockProver::new()));
        let verifier = MockVerifier::new();
        service.with_local_verifier(Box::new(verifier.clone()));
        let request = vector.to_request();

        // Prove once to warm caches, then time the full round trip.
        let response = service.prove(&request).expect("mock proves");
        let ns = best_ns(SAMPLES, || {
            let proved = service.prove(&request).expect("mock proves");
            let outcome = verifier
                .verify(&VerificationRequest::from_response(&proved))
                .expect("verifier runs");
            assert!(outcome.verified);
            let envelope = ProofEnvelope::from_response(&proved, vector.operation, "bench");
            let json = envelope.to_json().expect("envelope serializes");
            assert!(!json.is_empty());
        });
        println!(
            "{:<28} {}   ({} B proof)",
            vector.id,
            fmt_ns(ns),
            response.proof.bytes.len()
        );
    }
}
