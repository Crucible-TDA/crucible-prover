//! Stable contracts for the Crucible proof engine.
//!
//! This crate is the vocabulary shared by every crate in the workspace and by
//! the sibling repositories (`crucible-simulator`, `crucible-scenarios`). It
//! deliberately contains **no proving logic**: it defines *what* a proof
//! request is, *what* a provider must answer, and *what* a verifier checks.
//!
//! The module layout mirrors the repository plan:
//!
//! - [`circuit`] — circuit identities, versions, operations, and the field
//!   value model (public [`FieldValue`]s and never-exposed [`SecretValue`]s).
//! - [`proof_provider`] — the [`ProofProvider`] contract implemented by every
//!   backend (mock, Noir/UltraHonk), plus [`ProofRequest`] and
//!   [`ProofResponse`].
//! - [`prover`] — the client-facing facade contract [`Prover`] that
//!   `crucible-simulator` and `crucible-scenarios` depend on, so they never
//!   couple to a concrete backend.
//! - [`verifier`] — the [`Verifier`] contract with verification requests and
//!   structured [`VerificationOutcome`]s.
//!
//! # Dependency rule
//!
//! `crucible-simulator` MUST NOT require a concrete prover implementation. It
//! depends on the [`ProofProvider`]/[`Prover`] traits from this crate; the
//! implementations (mock, Noir, UltraHonk) can then evolve independently and
//! `crucible-scenarios` can pick mock or real proofs per test depth.
//!
//! # Privacy rule
//!
//! Values that must stay private ([`SecretValue`]) never implement `Debug`,
//! `Display`, or `Serialize`, and never appear in error messages. Requests
//! carrying private witness material only expose a redacted view via
//! [`ProofRequest::redacted`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod circuit;
pub mod proof_provider;
pub mod prover;
pub mod verifier;

pub use circuit::{
    CircuitId, CircuitMetadata, FieldError, FieldValue, Operation, OutputBag, PrivateWitnessBag,
    PublicInputBag, SecretValue, Version,
};
pub use proof_provider::{
    ArtifactChecksum, BackendId, ProofBlob, ProofFormat, ProofProvider, ProofRequest,
    ProofResponse, ProviderError, RequestId, RootDigest, StateReference, VerificationKeyId,
    WitnessError,
};
pub use verifier::{
    VerificationFailure, VerificationOutcome, VerificationRequest, Verifier, VerifierError,
};
