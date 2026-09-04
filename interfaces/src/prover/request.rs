//! Requests accepted by the [`Prover`](crate::prover::Prover) facade.
//!
//! The facade operates on the same wire types as providers — there is only
//! one proof request format in Crucible, which keeps serialization and
//! versioning in one place.

pub use crate::proof_provider::ProofRequest as ProveRequest;
