//! Proof-fixture regression net (see `proofs/README.md`).
//!
//! Committed envelope fixtures under `proofs/fixtures/` pin the wire format
//! and the verification contract with concrete material:
//!
//! - every valid fixture must round-trip through envelope JSON (v1 layout
//!   stability) and verify against the mock verifier with no hints;
//! - the invalid fixture (one proof byte flipped) must be rejected.
//!
//! No toolchain is required: the mock backend is deterministic, so these
//! fixtures are stable by construction and only change when the envelope
//! format or a catalog vector does.

use std::path::{Path, PathBuf};

use crucible_interfaces::{VerificationRequest, Verifier};
use crucible_mock::MockVerifier;
use crucible_proof_types::ProofEnvelope;

/// The repo's committed proof fixtures.
fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../proofs/fixtures")
}

fn envelopes_under(dir: &Path) -> Vec<(String, ProofEnvelope)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("fixtures dir readable") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path
                .file_stem()
                .expect("file name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&path).expect("fixture readable");
            let envelope =
                ProofEnvelope::from_json(&text).expect("fixture must parse as a v1 envelope");
            out.push((name, envelope));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Every valid fixture covers one operation and is in envelope format v1.
#[test]
fn valid_fixtures_cover_all_five_operations_in_v1() {
    let valid = envelopes_under(&fixtures_root().join("valid"));
    assert_eq!(valid.len(), 5, "one valid fixture per operation");
    let ops: Vec<&str> = valid.iter().map(|(_, e)| e.operation.as_str()).collect();
    for op in ["register", "deposit", "merge", "transfer", "withdraw"] {
        assert!(ops.contains(&op), "missing valid fixture for {op}");
    }
    for (name, envelope) in &valid {
        assert_eq!(envelope.version, 1, "{name}: envelope must be v1");
        assert_eq!(
            envelope.backend.as_str(),
            "mock",
            "{name}: fixtures are mock envelopes"
        );
    }
}

/// Valid fixtures round-trip through JSON and verify against the mock
/// verifier with no hints (the envelope is self-describing).
#[test]
fn valid_fixtures_round_trip_and_verify() {
    let verifier = MockVerifier::new();
    for (name, envelope) in envelopes_under(&fixtures_root().join("valid")) {
        // Serialization round trip must be lossless (v1 layout stability).
        let json = envelope.to_json().expect("serializes");
        let back = ProofEnvelope::from_json(&json).expect("parses");
        assert_eq!(back, envelope, "{name}: JSON round trip must be lossless");

        let request = VerificationRequest::new(
            envelope.metadata.request_id.clone(),
            envelope.proof.clone(),
            envelope.circuit.clone(),
            envelope.circuit_version,
            envelope.backend.clone(),
            envelope.verification_key_id.clone(),
            envelope.artifact_checksum.clone(),
            envelope.public_outputs.clone(),
            envelope.state_reference.clone(),
        );
        let outcome = verifier.verify(&request).expect("verifier runs");
        assert!(
            outcome.verified,
            "{name}: committed valid fixture must verify: {outcome}"
        );
    }
}

/// The tampered fixture must be rejected — this is the committed material
/// version of the malleability tests.
#[test]
fn invalid_fixture_is_rejected() {
    let invalid = envelopes_under(&fixtures_root().join("invalid"));
    assert!(!invalid.is_empty(), "at least one invalid fixture");
    let verifier = MockVerifier::new();
    for (name, envelope) in invalid {
        let request = VerificationRequest::new(
            envelope.metadata.request_id.clone(),
            envelope.proof.clone(),
            envelope.circuit.clone(),
            envelope.circuit_version,
            envelope.backend.clone(),
            envelope.verification_key_id.clone(),
            envelope.artifact_checksum.clone(),
            envelope.public_outputs.clone(),
            envelope.state_reference.clone(),
        );
        let outcome = verifier.verify(&request).expect("verifier runs");
        assert!(
            !outcome.verified,
            "{name}: tampered fixture must be rejected: {outcome}"
        );
    }
}
