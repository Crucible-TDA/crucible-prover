//! The [`MockProver`]: a deterministic, TEST-ONLY [`ProofProvider`].

use crucible_interfaces::{
    ArtifactChecksum, BackendId, CircuitId, Operation, ProofBlob, ProofFormat, ProofProvider,
    ProofRequest, ProofResponse, ProviderError, RequestId, StateReference, VerificationKeyId,
    Version,
};
use serde::{Deserialize, Serialize};

/// Canonical mock proof format tag (see [`ProofFormat::MOCK`]).
pub const MOCK_FORMAT: &str = "mock-envelope-v1";

/// Domain separator hashed into every mock proof so bytes from other
/// (hypothetical) envelope formats can never accidentally verify.
pub const DOMAIN: &[u8] = b"crucible-mock-envelope-v1";

/// Default per-instance mock key shared by default-constructed provers and
/// verifiers. Real backends derive this from actual key material; the mock
/// just needs prover and verifier to agree.
pub const DEFAULT_MOCK_KEY: &str = "crucible-mock-key-v1-test-only";

/// The self-describing payload embedded in every mock proof.
///
/// The mock performs no cryptography, so instead of an opaque proof blob it
/// stores the *context* the proof claims, in the clear. Verification then
/// checks each embedded field against the submitted context and can report a
/// precise [`crucible_interfaces::VerificationFailure`] reason — which is
/// what makes mock-based security tests meaningful.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MockEnvelope {
    /// Fixed marker (`"mock-envelope-v1"`) checked by the verifier.
    pub kind: String,
    /// Circuit this (fake) proof is for.
    pub circuit: CircuitId,
    /// Circuit version.
    pub circuit_version: Version,
    /// Backend id (always the mock backend).
    pub backend: BackendId,
    /// Verification key this proof claims to be valid under.
    pub verification_key_id: VerificationKeyId,
    /// Artifact checksum this proof claims to come from.
    pub artifact_checksum: ArtifactChecksum,
    /// Public outputs this (fake) proof commits to.
    pub public_outputs: crucible_interfaces::OutputBag,
    /// State this (fake) proof is bound to, when bound.
    pub state_reference: Option<StateReference>,
}

impl MockEnvelope {
    fn new(
        circuit: CircuitId,
        circuit_version: Version,
        backend: BackendId,
        verification_key_id: VerificationKeyId,
        artifact_checksum: ArtifactChecksum,
        public_outputs: crucible_interfaces::OutputBag,
        state_reference: Option<StateReference>,
    ) -> MockEnvelope {
        MockEnvelope {
            kind: MOCK_FORMAT.to_owned(),
            circuit,
            circuit_version,
            backend,
            verification_key_id,
            artifact_checksum,
            public_outputs,
            state_reference,
        }
    }

    /// Serializes the envelope to its canonical JSON byte form.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("mock envelope serialization cannot fail")
    }

    /// Parses an envelope from its canonical JSON byte form.
    pub fn from_bytes(bytes: &[u8]) -> Result<MockEnvelope, MockEnvelopeError> {
        let envelope: MockEnvelope = serde_json::from_slice(bytes).map_err(|e| {
            MockEnvelopeError::Malformed(format!("payload is not valid mock envelope JSON: {e}"))
        })?;
        if envelope.kind != MOCK_FORMAT {
            return Err(MockEnvelopeError::Malformed(format!(
                "unexpected mock envelope kind `{}`",
                envelope.kind
            )));
        }
        Ok(envelope)
    }
}

/// Why a mock proof payload could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MockEnvelopeError {
    /// The payload is not a well-formed mock envelope.
    #[error("{0}")]
    Malformed(String),
}

/// TEST ONLY. A deterministic, non-cryptographic [`ProofProvider`].
///
/// Every public-context field of the request is bound into the proof
/// envelope and then covered by a keyed digest, so:
///
/// - the **same request** always yields the **same proof bytes**
///   (deterministic golden fixtures);
/// - flipping any byte of a proof breaks its digest (tamper detection);
/// - verification under a different circuit, version, verification key,
///   artifact, public output set, or state reference fails with a precise
///   reason.
///
/// # NOT CRYPTOGRAPHICALLY SECURE
///
/// The digest is not a signature and the key is not secret material: anyone
/// with this source can forge a "proof". Test use only.
#[derive(Debug, Clone)]
pub struct MockProver {
    key: String,
}

impl Default for MockProver {
    fn default() -> MockProver {
        MockProver::new()
    }
}

impl MockProver {
    /// Creates a mock prover using the shared [`DEFAULT_MOCK_KEY`].
    pub fn new() -> MockProver {
        MockProver {
            key: DEFAULT_MOCK_KEY.to_owned(),
        }
    }

    /// Creates a mock prover with a caller-supplied key.
    ///
    /// The matching [`crate::MockVerifier`] must be constructed with the same
    /// key, or verification will report the proof as invalid.
    pub fn with_key(key: impl Into<String>) -> MockProver {
        MockProver { key: key.into() }
    }

    /// The key this prover signs (digests) its envelopes with.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The deterministic verification key id this mock assigns to a circuit.
    pub fn verification_key_id_for(circuit: &CircuitId, version: &Version) -> VerificationKeyId {
        VerificationKeyId::new(format!("mock-vk/{circuit}/{version}")).expect("id is valid")
    }

    /// The deterministic artifact checksum this mock assigns to a circuit.
    pub fn artifact_checksum_for(circuit: &CircuitId, version: &Version) -> ArtifactChecksum {
        ArtifactChecksum::from_bytes(format!("mock-artifact/{circuit}/{version}").as_bytes())
    }

    /// Builds the proof bytes for a request: canonical envelope JSON followed
    /// by a keyed digest over that JSON.
    pub fn build_proof_bytes(
        &self,
        request: &ProofRequest,
        verification_key_id: &VerificationKeyId,
        artifact_checksum: &ArtifactChecksum,
    ) -> Vec<u8> {
        let envelope = MockEnvelope::new(
            request.circuit.clone(),
            request.circuit_version,
            request.backend.clone(),
            verification_key_id.clone(),
            artifact_checksum.clone(),
            request.public_inputs.clone(),
            request.state_reference.clone(),
        );
        let payload = envelope.to_bytes();
        let mut bytes = payload.clone();
        bytes.extend_from_slice(&self.digest(&payload));
        bytes
    }

    fn digest(&self, payload: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(DOMAIN);
        hasher.update(payload);
        hasher.update(self.key.as_bytes());
        hasher.update([0u8]);
        let out = hasher.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&out);
        arr
    }
}

impl ProofProvider for MockProver {
    fn backend(&self) -> BackendId {
        BackendId::new(BackendId::MOCK).expect("mock backend id is valid")
    }

    fn supports(&self, _circuit: &CircuitId, _version: &Version) -> bool {
        // The mock has no circuit-specific knowledge and makes no
        // cryptographic claims, so any well-formed request is "provable".
        // Structural validity is enforced in generate() via request.validate().
        true
    }

    fn generate(&self, request: &ProofRequest) -> Result<ProofResponse, ProviderError> {
        request.validate()?;
        if request.backend != self.backend() {
            return Err(ProviderError::ProofGeneration {
                backend: request.backend.to_string(),
                reason: "request targets a different backend than the mock prover".to_owned(),
            });
        }
        // The mock cannot compute real circuit outputs, so it commits to the
        // request's public inputs as the outputs the circuit would produce.
        // Scenario code drives this by placing expected outputs in the
        // request's public inputs.
        let vk_id = Self::verification_key_id_for(&request.circuit, &request.circuit_version);
        let checksum = Self::artifact_checksum_for(&request.circuit, &request.circuit_version);
        let bytes = self.build_proof_bytes(request, &vk_id, &checksum);

        Ok(ProofResponse::new(
            request.request_id.clone(),
            request.circuit.clone(),
            request.circuit_version,
            self.backend(),
            ProofBlob::new(
                ProofFormat::new(MOCK_FORMAT).expect("mock format tag is valid"),
                bytes,
            ),
            request.public_inputs.clone(),
            vk_id,
            checksum,
            request.state_reference.clone(),
        ))
    }
}

/// The canonical operation set used by the mock fixtures.
pub const MOCK_OPERATIONS: [Operation; 5] = [
    Operation::Register,
    Operation::Deposit,
    Operation::Merge,
    Operation::Transfer,
    Operation::Withdraw,
];

/// Convenience alias used by fixtures that need a request id.
pub fn request_id(tag: &str) -> RequestId {
    RequestId::new(format!("mock-{tag}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crucible_interfaces::ProofProvider;

    #[test]
    fn envelope_round_trips_through_bytes() {
        let request = fixtures::transfer_request();
        let vk = MockProver::verification_key_id_for(&request.circuit, &request.circuit_version);
        let cs = MockProver::artifact_checksum_for(&request.circuit, &request.circuit_version);
        let envelope = MockEnvelope::new(
            request.circuit.clone(),
            request.circuit_version,
            request.backend.clone(),
            vk,
            cs,
            request.public_inputs.clone(),
            request.state_reference.clone(),
        );
        let parsed = MockEnvelope::from_bytes(&envelope.to_bytes()).unwrap();
        assert_eq!(parsed, envelope);
    }

    #[test]
    fn same_request_produces_same_proof_bytes() {
        let request = fixtures::transfer_request();
        let prover = MockProver::new();
        let a = prover.generate(&request).unwrap();
        let b = prover.generate(&request).unwrap();
        assert_eq!(a.proof.bytes, b.proof.bytes);
    }

    #[test]
    fn generate_is_deterministic_across_instances_with_same_key() {
        let request = fixtures::transfer_request();
        let a = MockProver::new().generate(&request).unwrap();
        let b = MockProver::new().generate(&request).unwrap();
        assert_eq!(a, b);
    }
}
