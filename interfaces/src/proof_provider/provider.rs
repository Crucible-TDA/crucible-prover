use crate::circuit::{CircuitId, Version};
use crate::proof_provider::{BackendId, ProofRequest, ProofResponse, ProviderError};

/// A backend that can generate proofs for a set of circuits.
///
/// This is the seam that keeps the rest of Crucible decoupled from any
/// concrete proving system:
///
/// ```text
/// simulator / scenarios
///          │  (depends on this trait only)
///          ▼
///    ProofProvider
///          ▲
///          │
///    MockProver · NoirProver · UltraHonkProver
/// ```
///
/// Implementations must be cheap to construct, `Send + Sync`, and report
/// their capabilities honestly through [`ProofProvider::supports`] so a
/// caller can fail fast instead of discovering unsupported circuits mid-run.
pub trait ProofProvider: Send + Sync {
    /// The backend identity this provider implements (see [`BackendId`]).
    fn backend(&self) -> BackendId;

    /// Whether this provider can prove `circuit` at `version` on its backend.
    fn supports(&self, circuit: &CircuitId, version: &Version) -> bool;

    /// Generates a proof for `request`.
    ///
    /// Implementations must first run [`ProofRequest::validate`] plus their
    /// own backend-specific checks (artifact availability, version pinning,
    /// required witness names) and return a structured [`ProviderError`] on
    /// failure. They must never log or embed private witness values in
    /// errors.
    fn generate(&self, request: &ProofRequest) -> Result<ProofResponse, ProviderError>;
}
