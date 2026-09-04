//! The [`ProverService`]: the concrete client-facing proving facade.

use crucible_interfaces::prover::{Prover, ProverError};
use crucible_interfaces::{
    ProofProvider, ProofRequest, ProofResponse, VerificationOutcome, Verifier,
};

use crate::backend::ProviderRegistry;
use crate::errors::CoreError;
use crate::proof;
use crate::verification;

/// A fully wired proving service: providers + a local verifier + provenance.
///
/// This is the type `crucible-simulator`, `crucible-scenarios`, and the CLI
/// construct once and reuse. It implements [`Prover`], so callers depend on
/// the interface and never touch concrete backends.
///
/// # Verification round-trip
///
/// The service can verify every proof it produces against its configured
/// local verifier. `prove_and_verify` refuses to return a proof that fails
/// its own round-trip: a rejected proof surfaces as
/// [`ProverError::NotVerified`].
pub struct ProverService {
    registry: ProviderRegistry,
    local_verifier: Option<Box<dyn Verifier>>,
    /// Producer label stamped into envelopes (e.g. `crucible-prover/0.1.0`).
    produced_by: String,
}

impl ProverService {
    /// Creates an empty service with no providers and no local verifier.
    pub fn new(produced_by: impl Into<String>) -> ProverService {
        ProverService {
            registry: ProviderRegistry::new(),
            local_verifier: None,
            produced_by: produced_by.into(),
        }
    }

    /// Registers a provider for its backend.
    pub fn register_provider(&mut self, provider: Box<dyn ProofProvider>) -> &mut Self {
        self.registry.register(provider);
        self
    }

    /// Sets the verifier used for the local round-trip.
    pub fn with_local_verifier(&mut self, verifier: Box<dyn Verifier>) -> &mut Self {
        self.local_verifier = Some(verifier);
        self
    }

    /// The registry backing this service.
    pub fn registry(&self) -> &ProviderRegistry {
        &self.registry
    }

    /// Producer label stamped into envelopes.
    pub fn produced_by(&self) -> &str {
        &self.produced_by
    }

    /// Runs structural pre-checks that do not require a backend: request
    /// validation plus circuit-level witness expectations. Returns a
    /// `CoreError` describing the first failure.
    fn preflight(&self, request: &ProofRequest) -> Result<(), CoreError> {
        request
            .validate()
            .map_err(|e| CoreError::InvalidRequest(e.to_string()))?;
        let missing = crate::witness::missing_private_names(request);
        if !missing.is_empty() {
            return Err(CoreError::InvalidRequest(format!(
                "operation {} requires private witness names {:?}",
                request.operation, missing
            )));
        }
        Ok(())
    }

    /// Generates a proof, dispatching to the registered provider, and wraps
    /// it in a [`crucible_proof_types::ProofEnvelope`].
    pub fn prove_enveloped(
        &self,
        request: &ProofRequest,
    ) -> Result<crucible_proof_types::ProofEnvelope, CoreError> {
        let response = self.prove_core(request)?;
        proof::assemble_envelope(&response, request.operation, self.produced_by.clone())
    }

    /// Generates a proof and verifies it locally, returning the envelope.
    pub fn prove_enveloped_and_verify(
        &self,
        request: &ProofRequest,
    ) -> Result<crucible_proof_types::ProofEnvelope, CoreError> {
        let (response, _) = self.prove_and_verify_core(request)?;
        proof::assemble_envelope(&response, request.operation, self.produced_by.clone())
    }

    fn prove_core(&self, request: &ProofRequest) -> Result<ProofResponse, CoreError> {
        self.preflight(request)?;
        self.registry.provide(request)
    }

    fn prove_and_verify_core(
        &self,
        request: &ProofRequest,
    ) -> Result<(ProofResponse, VerificationOutcome), CoreError> {
        let response = self.prove_core(request)?;
        let verifier = self
            .local_verifier
            .as_deref()
            .ok_or(CoreError::NoVerifier)?;
        let outcome = verification::verify_round_trip(verifier, &response)?;
        Ok((response, outcome))
    }
}

impl Prover for ProverService {
    fn prove(&self, request: &ProofRequest) -> Result<ProofResponse, ProverError> {
        self.prove_core(request).map_err(map_core_error)
    }

    fn prove_and_verify(
        &self,
        request: &ProofRequest,
    ) -> Result<(ProofResponse, VerificationOutcome), ProverError> {
        self.prove_and_verify_core(request).map_err(map_core_error)
    }

    fn local_verifier(&self) -> &dyn Verifier {
        // The default-trait prove_and_verify is overridden above, so this
        // accessor is only reached by callers that explicitly ask.
        self.local_verifier
            .as_deref()
            .expect("ProverService has no local verifier configured")
    }
}

fn map_core_error(error: CoreError) -> ProverError {
    match error {
        CoreError::UnknownBackend { .. } | CoreError::UnsupportedCircuit { .. } => {
            ProverError::NoProviderAvailable
        }
        CoreError::InvalidRequest(reason) => ProverError::Provider(reason),
        CoreError::Generation(reason) => ProverError::Provider(reason),
        CoreError::Envelope(reason) => ProverError::Provider(reason),
        CoreError::NotVerified(failure) => ProverError::NotVerified(failure),
        CoreError::NoVerifier => ProverError::VerificationFailed("no verifier configured".into()),
        CoreError::Internal(reason) => ProverError::Provider(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::VerificationRequest;
    use crucible_mock::{MockProver, MockVerifier, fixtures};

    fn mock_service() -> ProverService {
        let mut service = ProverService::new("test/0.1.0");
        service.register_provider(Box::new(MockProver::new()));
        service.with_local_verifier(Box::new(MockVerifier::new()));
        service
    }

    #[test]
    fn service_proves_and_round_trip_verifies() {
        let service = mock_service();
        for request in fixtures::all_valid_requests() {
            let (response, outcome) = service.prove_and_verify(&request).unwrap();
            assert!(outcome.verified);
            assert_eq!(response.circuit, request.circuit);
            assert_eq!(response.proof.format.as_str(), "mock-envelope-v1");
        }
    }

    #[test]
    fn service_returns_versioned_envelopes() {
        let service = mock_service();
        let request = fixtures::transfer_request();
        let envelope = service.prove_enveloped_and_verify(&request).unwrap();
        assert_eq!(envelope.circuit, request.circuit);
        assert_eq!(envelope.metadata.produced_by, "test/0.1.0");
        let json = envelope.to_json().unwrap();
        let back = crucible_proof_types::ProofEnvelope::from_json(&json).unwrap();
        assert_eq!(back, envelope);
    }

    #[test]
    fn service_rejects_requests_missing_witness_names() {
        let service = mock_service();
        let mut request = fixtures::transfer_request();
        // Drop one required private name (can't remove from the bag API, so
        // rebuild a structurally-invalid request via a fresh bag).
        request.witness = crucible_interfaces::PrivateWitnessBag::new();
        let err = service.prove(&request).unwrap_err();
        assert!(matches!(err, ProverError::Provider(_)));
    }

    #[test]
    fn service_without_verifier_cannot_round_trip() {
        let mut service = ProverService::new("test/0.1.0");
        service.register_provider(Box::new(MockProver::new()));
        let request = fixtures::transfer_request();
        let err = service.prove_and_verify(&request).unwrap_err();
        assert!(matches!(err, ProverError::VerificationFailed(_)));
    }

    #[test]
    fn service_without_provider_fails_fast() {
        let service = ProverService::new("test/0.1.0");
        let request = fixtures::transfer_request();
        let err = service.prove(&request).unwrap_err();
        assert_eq!(err, ProverError::NoProviderAvailable);
    }

    #[test]
    fn verification_request_building_is_lossless() {
        let service = mock_service();
        let request = fixtures::transfer_request();
        let response = service.prove(&request).unwrap();
        let verification = VerificationRequest::from_response(&response);
        assert_eq!(verification.circuit, response.circuit);
        assert_eq!(verification.public_outputs, response.public_outputs);
        assert_eq!(verification.state_reference, response.state_reference);
    }
}
