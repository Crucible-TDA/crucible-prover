/// Errors a verifier can return.
///
/// Note the distinction from [`crate::verifier::VerificationOutcome`]: a
/// *failed verification* (invalid/tampered proof) is a normal result with
/// `verified == false` and a structured failure reason. An [`VerifierError`]
/// means verification itself could not run (unsupported format, missing
/// verifier, unavailable backend).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerifierError {
    /// No verifier implementation exists for the requested format/backend.
    #[error("no verifier registered for format `{format}` on backend `{backend}`")]
    UnsupportedVerifier {
        /// The proof format that has no verifier.
        format: String,
        /// The backend that has no verifier.
        backend: String,
    },

    /// The verification request is malformed (e.g. wrong key id format).
    #[error("invalid verification request: {reason}")]
    InvalidRequest {
        /// Machine-readable reason.
        reason: String,
    },

    /// The verifier backend failed to run.
    #[error("verifier backend `{backend}` failed: {reason}")]
    VerificationUnavailable {
        /// The backend that failed.
        backend: String,
        /// Machine-readable reason.
        reason: String,
    },

    /// An unexpected internal failure.
    #[error("internal verifier error: {reason}")]
    Internal {
        /// Machine-readable reason.
        reason: String,
    },
}
