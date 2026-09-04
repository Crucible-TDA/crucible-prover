use crate::verifier::VerificationFailure;

/// Errors surfaced by the [`Prover`](crate::prover::Prover) facade.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProverError {
    /// No registered backend can serve the requested circuit/backend pair.
    #[error("no provider available for circuit/backend combination")]
    NoProviderAvailable,

    /// The underlying provider failed. Reason strings never contain witness
    /// values.
    #[error("provider error: {0}")]
    Provider(String),

    /// The local verification step could not run.
    #[error("local verification could not run: {0}")]
    VerificationFailed(String),

    /// The generated proof did not verify locally; the round-trip is
    /// rejected so callers cannot proceed with an unverified proof.
    #[error("generated proof did not verify locally: {0:?}")]
    NotVerified(Option<VerificationFailure>),
}
