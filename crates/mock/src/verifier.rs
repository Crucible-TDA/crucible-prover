//! The [`MockVerifier`]: a deterministic, TEST-ONLY [`Verifier`].

use crucible_interfaces::{
    BackendId, VerificationFailure, VerificationOutcome, VerificationRequest, Verifier,
    VerifierError,
};
use sha2::{Digest, Sha256};

use crate::prover::{DEFAULT_MOCK_KEY, DOMAIN, MOCK_FORMAT, MockEnvelope};

/// TEST ONLY. Verifies [`crate::MockProver`] envelopes.
///
/// Verification is *structural*: the envelope embedded in the proof carries
/// the full context it claims, and every field is compared against the
/// submitted [`VerificationRequest`]. The proof's keyed digest is checked
/// first, so tampered bytes are reported as [`VerificationFailure::InvalidProof`]
/// rather than as a context mismatch.
///
/// The failure reasons are meaningful only because the mock envelope is
/// self-describing; a real backend's opaque proof cannot say *why* it failed.
/// Security tests that assert specific reasons therefore encode the mock's
/// behavior and must be complemented by real-backend tests.
#[derive(Debug, Clone)]
pub struct MockVerifier {
    key: String,
}

impl Default for MockVerifier {
    fn default() -> MockVerifier {
        MockVerifier::new()
    }
}

impl MockVerifier {
    /// Creates a mock verifier using the shared [`DEFAULT_MOCK_KEY`].
    pub fn new() -> MockVerifier {
        MockVerifier {
            key: DEFAULT_MOCK_KEY.to_owned(),
        }
    }

    /// Creates a mock verifier with a caller-supplied key that must match the
    /// prover's key.
    pub fn with_key(key: impl Into<String>) -> MockVerifier {
        MockVerifier { key: key.into() }
    }

    /// Splits proof bytes into `(payload, digest)` and validates the digest.
    fn verify_digest(&self, payload: &[u8], digest: &[u8]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN);
        hasher.update(payload);
        hasher.update(self.key.as_bytes());
        hasher.update([0u8]);
        let out = hasher.finalize();
        digest == out.as_slice()
    }
}

impl Verifier for MockVerifier {
    fn backend(&self) -> BackendId {
        BackendId::new(BackendId::MOCK).expect("mock backend id is valid")
    }

    fn verify(&self, request: &VerificationRequest) -> Result<VerificationOutcome, VerifierError> {
        // 1. Format routing: this verifier only understands mock envelopes.
        if request.proof.format.as_str() != MOCK_FORMAT
            || request.backend.as_str() != BackendId::MOCK
        {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::BackendMismatch,
            ));
        }

        // 2. Digest check: any tampered byte fails here, before any field is
        //    compared, so a modified proof is reported as InvalidProof.
        let (payload, digest) = match request.proof.bytes.len().checked_sub(32) {
            Some(split) => (&request.proof.bytes[..split], &request.proof.bytes[split..]),
            None => {
                return Ok(VerificationOutcome::rejected(
                    VerificationFailure::InvalidProof,
                ));
            }
        };
        if !self.verify_digest(payload, digest) {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::InvalidProof,
            ));
        }
        let envelope = match MockEnvelope::from_bytes(payload) {
            Ok(envelope) => envelope,
            Err(_) => {
                return Ok(VerificationOutcome::rejected(
                    VerificationFailure::InvalidProof,
                ));
            }
        };

        // 3. Field-by-field context comparison against the submitted request.
        if envelope.backend != request.backend {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::BackendMismatch,
            ));
        }
        if envelope.circuit != request.circuit {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::CircuitMismatch,
            ));
        }
        if envelope.circuit_version != request.circuit_version {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::VersionMismatch,
            ));
        }
        if envelope.verification_key_id != request.verification_key_id {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::WrongVerificationKey,
            ));
        }
        if envelope.artifact_checksum != request.artifact_checksum {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::ArtifactChecksumMismatch,
            ));
        }
        if envelope.public_outputs != request.public_outputs {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::PublicOutputMismatch,
            ));
        }
        match (&envelope.state_reference, &request.state_reference) {
            (None, None) => {}
            (Some(embedded), Some(submitted)) if embedded == submitted => {}
            // The proof requires a binding the request lacks, or the request
            // demands a binding the proof never made.
            (None, Some(_)) | (Some(_), None) => {
                return Ok(VerificationOutcome::rejected(
                    VerificationFailure::MissingStateBinding,
                ));
            }
            // Both present but unequal: stale state or replay attempt.
            _ => {
                return Ok(VerificationOutcome::rejected(
                    VerificationFailure::StateReferenceMismatch,
                ));
            }
        }

        Ok(VerificationOutcome::verified())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockProver, fixtures};
    use crucible_interfaces::{
        FieldValue, ProofProvider, ProofResponse, RootDigest, StateReference, VerificationRequest,
    };

    #[test]
    fn valid_proof_verifies() {
        let prover = MockProver::new();
        let verifier = MockVerifier::new();
        let request = fixtures::transfer_request();
        let response = prover.generate(&request).unwrap();
        let verification = VerificationRequest::from_response(&response);
        let outcome = verifier.verify(&verification).unwrap();
        assert!(outcome.verified, "mock proof should verify: {outcome}");
    }

    #[test]
    fn wrong_key_rejects_proof_as_invalid() {
        let prover = MockProver::new();
        let verifier = MockVerifier::with_key("a-different-key");
        let request = fixtures::transfer_request();
        let response = prover.generate(&request).unwrap();
        let verification = VerificationRequest::from_response(&response);
        let outcome = verifier.verify(&verification).unwrap();
        assert!(outcome.rejected_with(VerificationFailure::InvalidProof));
    }

    #[test]
    fn tampered_proof_is_rejected() {
        let prover = MockProver::new();
        let verifier = MockVerifier::new();
        let request = fixtures::transfer_request();
        let response = prover.generate(&request).unwrap();
        let mut tampered = response.clone();
        let last = tampered.proof.bytes.len() - 1;
        tampered.proof.bytes[last] ^= 0x01;
        let verification = VerificationRequest::from_response(&tampered);
        let outcome = verifier.verify(&verification).unwrap();
        assert!(outcome.rejected_with(VerificationFailure::InvalidProof));
    }

    #[test]
    fn wrong_context_fields_produce_precise_reasons() {
        let prover = MockProver::new();
        let verifier = MockVerifier::new();
        let request = fixtures::transfer_request();
        let response = prover.generate(&request).unwrap();

        type Case = (fn(&mut ProofResponse), VerificationFailure);
        let cases: Vec<Case> = vec![
            (
                |r| {
                    r.circuit = crucible_interfaces::CircuitId::new("withdraw").unwrap();
                },
                VerificationFailure::CircuitMismatch,
            ),
            (
                |r| {
                    r.circuit_version = crucible_interfaces::Version::new(9, 9, 9);
                },
                VerificationFailure::VersionMismatch,
            ),
            (
                |r| {
                    r.verification_key_id =
                        crucible_interfaces::VerificationKeyId::new("mock-vk/wrong").unwrap();
                },
                VerificationFailure::WrongVerificationKey,
            ),
            (
                |r| {
                    r.artifact_checksum =
                        crucible_interfaces::ArtifactChecksum::from_bytes(b"different artifact");
                },
                VerificationFailure::ArtifactChecksumMismatch,
            ),
            (
                |r| {
                    r.public_outputs
                        .insert("amount", FieldValue::from_hex("deadbeef").unwrap())
                        .unwrap();
                },
                VerificationFailure::PublicOutputMismatch,
            ),
        ];
        for (mutate, expected) in cases {
            let mut mutated = response.clone();
            mutate(&mut mutated);
            let verification = VerificationRequest::from_response(&mutated);
            let outcome = verifier.verify(&verification).unwrap();
            assert!(
                outcome.rejected_with(expected),
                "expected {expected:?}, got {outcome:?}"
            );
        }
    }

    #[test]
    fn stale_state_and_replay_are_rejected() {
        let prover = MockProver::new();
        let verifier = MockVerifier::new();
        let request = fixtures::transfer_request();
        let response = prover.generate(&request).unwrap();

        // Submit the proof against a different (later) state root.
        let mut stale = response.clone();
        stale.state_reference = Some(StateReference::new(
            RootDigest::from_hex(&"cd".repeat(32)).unwrap(),
            8,
        ));
        let verification = VerificationRequest::from_response(&stale);
        let outcome = verifier.verify(&verification).unwrap();
        assert!(outcome.rejected_with(VerificationFailure::StateReferenceMismatch));

        // Drop the state binding entirely: the operation requires it.
        let mut unbound = response.clone();
        unbound.state_reference = None;
        let verification = VerificationRequest::from_response(&unbound);
        let outcome = verifier.verify(&verification).unwrap();
        assert!(outcome.rejected_with(VerificationFailure::MissingStateBinding));
    }

    #[test]
    fn operation_helpers_produce_verifiable_round_trips() {
        let prover = MockProver::new();
        let verifier = MockVerifier::new();
        for request in fixtures::all_valid_requests() {
            let response = prover.generate(&request).unwrap();
            assert_eq!(response.circuit, request.circuit);
            let outcome = verifier
                .verify(&VerificationRequest::from_response(&response))
                .unwrap();
            assert!(
                outcome.verified,
                "{} request did not verify: {outcome}",
                request.operation
            );
        }
    }
}
