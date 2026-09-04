//! The client-facing proving facade.
//!
//! Backends implement [`crate::proof_provider::ProofProvider`]; clients
//! depend on [`Prover`] here, which dispatches to the right provider and can
//! run the local verification round-trip.

// See circuit/mod.rs for why the module-inception lint is disabled: the
// nested file names intentionally mirror the repository plan.
mod errors;
#[allow(clippy::module_inception)]
mod prover;
mod request;
mod response;

pub use errors::ProverError;
pub use prover::Prover;
pub use request::ProveRequest;
pub use response::ProveResponse;
