//! Errors produced by the verification service.

/// Errors that occur while *dispatching* a verification.
///
/// A failed verification (tampered proof, wrong context) is a normal
/// [`crucible_interfaces::VerificationOutcome`], not an error. These errors
/// mean the service could not run verification at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifierServiceError {
    /// No verifier is registered for the requested backend.
    #[error("no verifier is registered for backend `{backend}`")]
    UnknownBackend {
        /// The backend that has no verifier.
        backend: String,
    },

    /// A registered verifier failed to run.
    #[error("verifier `{label}` failed: {reason}")]
    VerifierFailed {
        /// Label of the verifier that failed.
        label: String,
        /// Machine-readable reason.
        reason: String,
    },

    /// Internal inconsistency in the service.
    #[error("internal verifier service error: {reason}")]
    Internal {
        /// Machine-readable reason.
        reason: String,
    },
}
