use crate::proof_provider::BackendId;
use crate::verifier::{VerificationOutcome, VerificationRequest, VerifierError};

/// Something that can verify proofs of one format.
///
/// Implementations exist per backend and target: a local mock verifier, a
/// local UltraHonk verifier, and a Soroban verifier that checks proofs
/// through an on-chain contract. Keeping them behind one trait makes it
/// possible to generate a proof, verify it locally, verify it on Soroban,
/// and detect discrepancies — which is exactly what Crucible must not assume
/// away.
pub trait Verifier: Send + Sync {
    /// Backend this verifier understands (see [`BackendId`]).
    fn backend(&self) -> BackendId;

    /// Verifies `request`.
    ///
    /// Implementations must check the proof under the *submitted* context:
    /// circuit, version, verification key, public outputs, and state
    /// reference. A proof that is valid but for a different context must come
    /// back as a rejected [`VerificationOutcome`], not as an error.
    fn verify(&self, request: &VerificationRequest) -> Result<VerificationOutcome, VerifierError>;
}
