//! Responses returned by the [`Prover`](crate::prover::Prover) facade.
//!
//! The facade returns the same traceable [`ProofResponse`] a provider
//! produces; no separate client-side response type exists.

pub use crate::proof_provider::ProofResponse as ProveResponse;
