//! Versioned, serializable proof types.
//!
//! [`crucible-interfaces`] defines the in-memory contracts; this crate
//! defines the **wire format** — the stable, versioned representation that
//! proofs are stored and exchanged in. The centerpiece is the
//! [`ProofEnvelope`](common::ProofEnvelope): a self-describing container that
//! answers *which circuit, which version, which backend, which verification
//! key, and which artifact checksum* produced a proof, so every stored proof
//! is traceable and reproducible.
//!
//! # Versioning policy
//!
//! The envelope format itself carries an [`EnvelopeVersion`](common::EnvelopeVersion).
//! Parsing rejects envelopes with a newer version than this crate
//! understands ([`EnvelopeError::UnsupportedVersion`](errors::EnvelopeError::UnsupportedVersion)),
//! so old tooling can never silently misinterpret newer proofs.
//!
//! # Per-operation typed modules
//!
//! The repository plan gives this crate one module per operation
//! (`register`, `deposit`, `merge`, `transfer`, `withdraw`). Those modules
//! will contain the *typed public-input layouts* of each circuit and will
//! land together with the circuits themselves (see `circuits/`), because the
//! exact public-input boundary must follow the Confidential Token circuit
//! specification rather than be invented here.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod common;
pub mod errors;

pub use common::{EnvelopeMetadata, EnvelopeVersion, ProofEnvelope};
pub use errors::EnvelopeError;
