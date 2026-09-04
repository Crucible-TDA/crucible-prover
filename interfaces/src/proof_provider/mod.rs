//! The provider contract and its wire types.
//!
//! A *proof provider* turns a [`ProofRequest`] into a [`ProofResponse`].
//! Concrete providers — mock, Noir/UltraHonk — implement [`ProofProvider`];
//! clients (`crucible-simulator`, `crucible-scenarios`) depend only on this
//! trait, never on a backend.

mod errors;
mod proof;
mod provider;
mod request;

pub use errors::ProviderError;
pub use proof::{ArtifactChecksum, ProofBlob, ProofFormat, ProofResponse, VerificationKeyId};
pub use provider::ProofProvider;
pub use request::{BackendId, ProofRequest, RequestId, RootDigest, StateReference, WitnessError};
