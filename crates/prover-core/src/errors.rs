//! Errors produced by the proving orchestration layer.

use crucible_interfaces::{BackendId, CircuitId, VerificationFailure, Version};

/// Errors that can occur while orchestrating a proof.
///
/// Error messages carry identifiers and reasons only — never witness values.
/// If an underlying error message could contain private material, it is
/// classified before being wrapped.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// No provider is registered for the requested backend.
    #[error("no provider is registered for backend `{backend}`")]
    UnknownBackend {
        /// The requested backend.
        backend: BackendId,
    },

    /// A registered provider does not support the requested circuit/version.
    #[error("backend `{backend}` does not support circuit `{circuit}` at version {version}")]
    UnsupportedCircuit {
        /// The backend that was asked.
        backend: BackendId,
        /// The unsupported circuit.
        circuit: CircuitId,
        /// The unsupported version.
        version: Version,
    },

    /// The request failed structural validation.
    #[error("invalid proof request: {0}")]
    InvalidRequest(String),

    /// Proof generation failed in the provider.
    #[error("proof generation failed: {0}")]
    Generation(String),

    /// The proof envelope could not be assembled or serialized.
    #[error("proof envelope error: {0}")]
    Envelope(String),

    /// A proof was produced but failed the local verification round-trip.
    #[error("proof did not verify locally: {0:?}")]
    NotVerified(Option<VerificationFailure>),

    /// No verifier is available for the round-trip.
    #[error("no verifier is available to run the round-trip check")]
    NoVerifier,

    /// An internal invariant was violated.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<crucible_interfaces::ProviderError> for CoreError {
    fn from(error: crucible_interfaces::ProviderError) -> CoreError {
        CoreError::Generation(error.to_string())
    }
}
