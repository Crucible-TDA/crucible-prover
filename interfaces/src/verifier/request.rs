use crate::circuit::{CircuitId, OutputBag, Version};
use crate::proof_provider::{
    ArtifactChecksum, BackendId, ProofBlob, RequestId, StateReference, VerificationKeyId,
};

/// Everything a verifier needs to check one proof.
///
/// A [`VerificationRequest`] deliberately mirrors an (untrusted)
/// [`crate::proof_provider::ProofResponse`] plus the expected public context:
/// a caller submits the proof artifact *and* the public outputs/state it
/// claims to prove, and the verifier answers whether the proof is valid for
/// exactly that context. This is what makes stale-state and replay misuse
/// detectable: submit a valid proof with a *different* state reference or
/// public output set and verification must fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRequest {
    /// Id of the original proof request, for traceability.
    pub request_id: RequestId,
    /// The proof bytes and their format.
    pub proof: ProofBlob,
    /// Circuit the proof claims to be for.
    pub circuit: CircuitId,
    /// Circuit version the proof claims to be for.
    pub circuit_version: Version,
    /// Backend the proof claims to come from.
    pub backend: BackendId,
    /// Verification key the proof claims to be valid under.
    pub verification_key_id: VerificationKeyId,
    /// Checksum of the artifact the proof claims to come from.
    pub artifact_checksum: ArtifactChecksum,
    /// Public outputs the proof must verify against.
    pub public_outputs: OutputBag,
    /// State reference the proof must verify against.
    pub state_reference: Option<StateReference>,
}

impl VerificationRequest {
    /// Creates a verification request.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: RequestId,
        proof: ProofBlob,
        circuit: CircuitId,
        circuit_version: Version,
        backend: BackendId,
        verification_key_id: VerificationKeyId,
        artifact_checksum: ArtifactChecksum,
        public_outputs: OutputBag,
        state_reference: Option<StateReference>,
    ) -> VerificationRequest {
        VerificationRequest {
            request_id,
            proof,
            circuit,
            circuit_version,
            backend,
            verification_key_id,
            artifact_checksum,
            public_outputs,
            state_reference,
        }
    }

    /// Builds a verification request directly from a [`ProofResponse`].
    ///
    /// This is the common "verify the proof I just generated" path. For
    /// tamper tests, mutate the returned request's fields before verifying.
    pub fn from_response(response: &crate::proof_provider::ProofResponse) -> VerificationRequest {
        VerificationRequest::new(
            response.request_id.clone(),
            response.proof.clone(),
            response.circuit.clone(),
            response.circuit_version,
            response.backend.clone(),
            response.verification_key_id.clone(),
            response.artifact_checksum.clone(),
            response.public_outputs.clone(),
            response.state_reference.clone(),
        )
    }
}
