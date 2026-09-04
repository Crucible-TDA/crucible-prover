use crate::circuit::{CircuitId, Version};
use crate::proof_provider::WitnessError;

/// Errors a proof provider can return.
///
/// # Privacy
///
/// Error variants carry identifiers and reason strings only — never witness
/// values. If a backend failure message would include private material, the
/// provider must truncate or classify it before converting into
/// [`ProviderError::Internal`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    /// The request failed structural validation.
    #[error(transparent)]
    InvalidRequest(#[from] WitnessError),

    /// The provider does not support the requested circuit.
    #[error(
        "provider backend `{backend}` does not support circuit `{circuit}` at version {version}"
    )]
    UnsupportedCircuit {
        /// The requested backend.
        backend: String,
        /// The unsupported circuit.
        circuit: CircuitId,
        /// The unsupported version.
        version: Version,
    },

    /// The compiled artifact required for proving is unavailable.
    #[error(
        "compiled artifact for circuit `{circuit}` at version {version} is unavailable on backend `{backend}`"
    )]
    ArtifactUnavailable {
        /// The backend that needs the artifact.
        backend: String,
        /// The circuit whose artifact is missing.
        circuit: CircuitId,
        /// The version whose artifact is missing.
        version: Version,
    },

    /// The artifact failed its integrity check (checksum mismatch).
    #[error("artifact for circuit `{circuit}` at version {version} failed integrity verification")]
    ArtifactIntegrity {
        /// The circuit whose artifact was rejected.
        circuit: CircuitId,
        /// The version whose artifact was rejected.
        version: Version,
    },

    /// The external proving toolchain failed or is not installed.
    #[error("proving backend `{backend}` is unavailable: {reason}")]
    BackendUnavailable {
        /// The backend that is unavailable.
        backend: String,
        /// Machine-readable reason.
        reason: String,
    },

    /// The backend failed while generating the proof.
    #[error("proof generation failed on backend `{backend}`: {reason}")]
    ProofGeneration {
        /// The backend that failed.
        backend: String,
        /// Machine-readable reason. Must never contain witness values.
        reason: String,
    },

    /// An unexpected internal failure.
    #[error("internal provider error: {reason}")]
    Internal {
        /// Machine-readable reason. Must never contain witness values.
        reason: String,
    },
}
