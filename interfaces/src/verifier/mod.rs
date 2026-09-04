//! The verifier contract and verification result model.
//!
//! Verification is deliberately a *separate* contract from proving: proofs
//! generated locally must be checkable locally (mock, UltraHonk) and against
//! a Soroban verifier contract, and discrepancies between those paths are
//! themselves bugs that Crucible exists to catch.

// See circuit/mod.rs for why the module-inception lint is disabled.
mod errors;
mod request;
mod response;
#[allow(clippy::module_inception)]
mod verifier;

pub use errors::VerifierError;
pub use request::VerificationRequest;
pub use response::{VerificationFailure, VerificationOutcome};
pub use verifier::Verifier;
