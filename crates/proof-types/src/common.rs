//! The versioned proof envelope and its metadata.

use serde::{Deserialize, Serialize};

use crate::errors::EnvelopeError;
use crucible_interfaces::{
    ArtifactChecksum, BackendId, CircuitId, Operation, OutputBag, ProofBlob, ProofResponse,
    RequestId, StateReference, VerificationKeyId, Version,
};

/// The envelope wire-format version this crate produces and understands.
pub type EnvelopeVersion = u32;

/// Envelope format v1 (current).
pub const ENVELOPE_V1: EnvelopeVersion = 1;

/// Provenance metadata attached to every envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeMetadata {
    /// Id of the proof request that produced the proof.
    pub request_id: RequestId,
    /// Free-form producer label (e.g. `crucible-prover/0.1.0`, `mock`).
    pub produced_by: String,
}

impl EnvelopeMetadata {
    /// Creates envelope metadata.
    pub fn new(request_id: RequestId, produced_by: impl Into<String>) -> EnvelopeMetadata {
        EnvelopeMetadata {
            request_id,
            produced_by: produced_by.into(),
        }
    }
}

/// Self-describing, versioned container for one proof.
///
/// ```text
/// ProofEnvelope {
///     version, circuit, backend,
///     proof, public_outputs,
///     verification_key_id, artifact_checksum,
///     state_reference, metadata
/// }
/// ```
///
/// The envelope is the unit of storage, exchange, and cross-language
/// fixture testing: whatever consumes it can decide how to verify without
/// guessing at the provenance of the bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofEnvelope {
    /// Envelope format version (see [`ENVELOPE_V1`]).
    pub version: EnvelopeVersion,
    /// Operation this proof implements.
    pub operation: Operation,
    /// Circuit that produced the proof.
    pub circuit: CircuitId,
    /// Circuit version the proof is valid for.
    pub circuit_version: Version,
    /// Backend that generated the proof.
    pub backend: BackendId,
    /// Proof bytes with their format tag.
    pub proof: ProofBlob,
    /// Public outputs the proof commits to.
    pub public_outputs: OutputBag,
    /// Verification key the proof must be checked against.
    pub verification_key_id: VerificationKeyId,
    /// SHA-256 checksum of the artifact that produced the proof.
    pub artifact_checksum: ArtifactChecksum,
    /// State the proof is bound to, when bound.
    pub state_reference: Option<StateReference>,
    /// Provenance metadata.
    pub metadata: EnvelopeMetadata,
}

impl ProofEnvelope {
    /// Wraps a proof response into the current envelope format.
    ///
    /// `operation` is taken explicitly (not derived from the circuit id) so a
    /// response can never be mislabeled when custom circuit ids exist.
    /// `produced_by` names the producer for provenance (e.g.
    /// `crucible-prover/0.1.0` or `mock/0.1.0`).
    pub fn from_response(
        response: &ProofResponse,
        operation: Operation,
        produced_by: impl Into<String>,
    ) -> ProofEnvelope {
        ProofEnvelope {
            version: ENVELOPE_V1,
            operation,
            circuit: response.circuit.clone(),
            circuit_version: response.circuit_version,
            backend: response.backend.clone(),
            proof: response.proof.clone(),
            public_outputs: response.public_outputs.clone(),
            verification_key_id: response.verification_key_id.clone(),
            artifact_checksum: response.artifact_checksum.clone(),
            state_reference: response.state_reference.clone(),
            metadata: EnvelopeMetadata::new(response.request_id.clone(), produced_by),
        }
    }

    /// Serializes the envelope to compact JSON.
    pub fn to_json(&self) -> Result<String, EnvelopeError> {
        serde_json::to_string(self).map_err(EnvelopeError::from)
    }

    /// Serializes the envelope to pretty-printed JSON (for fixtures).
    pub fn to_json_pretty(&self) -> Result<String, EnvelopeError> {
        serde_json::to_string_pretty(self).map_err(EnvelopeError::from)
    }

    /// Parses an envelope, rejecting versions this crate cannot handle.
    pub fn from_json(json: &str) -> Result<ProofEnvelope, EnvelopeError> {
        let envelope: ProofEnvelope = serde_json::from_str(json).map_err(EnvelopeError::from)?;
        if envelope.version > ENVELOPE_V1 {
            return Err(EnvelopeError::UnsupportedVersion {
                found: envelope.version,
                supported: ENVELOPE_V1,
            });
        }
        Ok(envelope)
    }

    /// The root digest this proof is bound to, when bound.
    pub fn state_root(&self) -> Option<&crucible_interfaces::RootDigest> {
        self.state_reference.as_ref().map(|s| &s.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::{
        BackendId, CircuitId, FieldValue, Operation, PrivateWitnessBag, ProofFormat, RootDigest,
        SecretValue,
    };

    fn sample_response() -> ProofResponse {
        let mut private = PrivateWitnessBag::new();
        private
            .insert("sender_sk", SecretValue::from_hex("0x11").unwrap())
            .unwrap();
        let mut outputs = OutputBag::new();
        outputs
            .insert("new_commitment", FieldValue::from_hex("deadbeef").unwrap())
            .unwrap();
        ProofResponse::new(
            RequestId::new("req-42"),
            CircuitId::for_operation(Operation::Transfer),
            Version::v0_1(),
            BackendId::new(BackendId::MOCK).unwrap(),
            ProofBlob::new(
                ProofFormat::new(ProofFormat::MOCK).unwrap(),
                vec![0xde, 0xad],
            ),
            outputs,
            VerificationKeyId::new("vk-mock-transfer-0.1.0").unwrap(),
            ArtifactChecksum::from_bytes(b"artifact"),
            Some(StateReference::new(
                RootDigest::from_hex(&"ab".repeat(32)).unwrap(),
                7,
            )),
        )
    }

    #[test]
    fn envelope_round_trips_through_json() {
        let response = sample_response();
        let envelope = ProofEnvelope::from_response(&response, Operation::Transfer, "mock/0.1.0");
        let json = envelope.to_json().unwrap();
        let back = ProofEnvelope::from_json(&json).unwrap();
        assert_eq!(back, envelope);
        assert_eq!(back.state_root().unwrap().as_hex(), "ab".repeat(32));
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"operation\":\"transfer\""));
    }

    #[test]
    fn envelope_rejects_future_versions() {
        let response = sample_response();
        let envelope = ProofEnvelope::from_response(&response, Operation::Transfer, "mock/0.1.0");
        let json = envelope.to_json().unwrap();
        // Rewrite the version to a future value.
        let tampered = json.replacen("\"version\":1", "\"version\":99", 1);
        assert_eq!(
            ProofEnvelope::from_json(&tampered).unwrap_err(),
            EnvelopeError::UnsupportedVersion {
                found: 99,
                supported: ENVELOPE_V1
            }
        );
    }

    #[test]
    fn envelope_rejects_malformed_json() {
        assert!(matches!(
            ProofEnvelope::from_json("{not json"),
            Err(EnvelopeError::Encoding(_))
        ));
    }

    #[test]
    fn envelope_serialization_is_deterministic() {
        let response = sample_response();
        let a = ProofEnvelope::from_response(&response, Operation::Transfer, "mock/0.1.0");
        let b = ProofEnvelope::from_response(&response, Operation::Transfer, "mock/0.1.0");
        assert_eq!(a.to_json().unwrap(), b.to_json().unwrap());
    }
}
