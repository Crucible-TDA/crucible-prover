//! Verification service and dispatch.
//!
//! The [`Verifier`](crucible_interfaces::Verifier) trait defines *what* a
//! verifier checks. This crate owns *which* verifier runs, and — crucially —
//! the ability to run **several** verifiers for the same proof and compare
//! their answers.
//!
//! That last capability is a core Crucible responsibility: local verification
//! and on-chain (Soroban) verification must not be assumed equivalent without
//! testing. [`service::VerificationService`] therefore supports registering
//! multiple verifiers per backend (e.g. `local-ultrahonk` and `soroban`,
//! both implementing the same backend), verifying against all of them, and
//! reporting a [`report::VerificationReport`] that states whether they
//! agreed.
//!
//! ```text
//! proof ──► VerificationService
//!              ├── MockVerifier        (backend "mock")
//!              ├── UltraHonkVerifier   (backend "ultrahonk", local)
//!              └── SorobanVerifier     (backend "ultrahonk", on-chain)
//!                        │
//!                        ▼
//!               VerificationReport
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod errors;
pub mod report;
pub mod service;

pub use errors::VerifierServiceError;
pub use report::{VerificationReport, VerifierResult};
pub use service::{VerificationService, VerifierRegistration};
