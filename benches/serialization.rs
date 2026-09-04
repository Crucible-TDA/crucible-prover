//! Envelope serialization bench: to_json and from_json round trips.
//!
//! The envelope is the unit of storage and exchange, so serialize/parse
//! throughput matters for fixture pipelines, scenario suites, and any
//! consumer that moves proofs in bulk.

use crucible_benches::{best_ns, fmt_ns, valid_vectors};
use crucible_interfaces::prover::Prover;
use crucible_mock::MockProver;
use crucible_proof_types::ProofEnvelope;
use crucible_prover_core::ProverService;

fn main() {
    const SAMPLES: usize = 500;
    println!("envelope serialization round trip, best of {SAMPLES}");
    println!("{:<28} ns/op", "vector");

    let mut service = ProverService::new("crucible-benches/0.1.0");
    service.register_provider(Box::new(MockProver::new()));
    for vector in valid_vectors() {
        let request = vector.to_request();
        let response = service.prove(&request).expect("mock proves");
        let envelope = ProofEnvelope::from_response(&response, vector.operation, "bench");
        let json = envelope.to_json().expect("serializes");

        let ns = best_ns(SAMPLES, || {
            let text = envelope.to_json().expect("serializes");
            let parsed = ProofEnvelope::from_json(&text).expect("parses");
            assert_eq!(parsed, envelope);
        });
        println!("{:<28} {}   ({} B json)", vector.id, fmt_ns(ns), json.len());
    }
}
