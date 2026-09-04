use crate::proof_provider::{ProofRequest, ProofResponse, ProviderError};
use crate::prover::ProverError;
use crate::verifier::{VerificationOutcome, VerificationRequest};

/// The client-facing proving entry point.
///
/// [`ProofProvider`](crate::proof_provider::ProofProvider) is the contract
/// each *backend* implements; [`Prover`] is the contract clients
/// (`crucible-simulator`, `crucible-scenarios`, the CLI) depend on. A `Prover`
/// implementation selects a provider by the request's circuit/backend and can
/// optionally run the local verification round-trip, so callers get one
/// stable surface instead of backend dispatch logic.
///
/// This is the trait `crucible-simulator` and `crucible-scenarios` should
/// depend on; they must never import a concrete backend crate.
pub trait Prover: Send + Sync {
    /// Generates a proof for `request`, dispatching to the right backend.
    fn prove(&self, request: &ProofRequest) -> Result<ProofResponse, ProverError>;

    /// Generates a proof and immediately verifies it locally.
    ///
    /// Convenience for development and tests. Returns the proof and its
    /// outcome; a rejected outcome here is surfaced as [`ProverError::NotVerified`]
    /// so callers cannot silently continue with an unverified proof.
    fn prove_and_verify(
        &self,
        request: &ProofRequest,
    ) -> Result<(ProofResponse, VerificationOutcome), ProverError> {
        let response = self.prove(request)?;
        let outcome = self
            .local_verifier()
            .verify(&VerificationRequest::from_response(&response))
            .map_err(|e| ProverError::VerificationFailed(e.to_string()))?;
        if !outcome.verified {
            return Err(ProverError::NotVerified(outcome.failure));
        }
        Ok((response, outcome))
    }

    /// The local verifier paired with this prover (mock↔mock, noir↔mock
    /// during development, ultrahonk↔ultrahonk in production).
    fn local_verifier(&self) -> &dyn crate::verifier::Verifier;
}

/// Helper trait implementations cannot provide by default.
impl<T> Prover for Box<T>
where
    T: Prover + ?Sized,
{
    fn prove(&self, request: &ProofRequest) -> Result<ProofResponse, ProverError> {
        (**self).prove(request)
    }

    fn local_verifier(&self) -> &dyn crate::verifier::Verifier {
        (**self).local_verifier()
    }
}

/// Converts a backend provider error into the facade error type.
impl From<ProviderError> for ProverError {
    fn from(error: ProviderError) -> ProverError {
        ProverError::Provider(error.to_string())
    }
}
