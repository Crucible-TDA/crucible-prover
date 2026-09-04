//! UltraHonk backend knowledge.
//!
//! UltraHonk is the proving system behind Stellar Confidential Tokens: Noir
//! circuits compiled with `nargo`, proved with the Barretenberg backend, and
//! verified on-chain by an UltraHonk verifier. This crate owns the
//! UltraHonk-specific *knowledge* — proof format tags, verification key
//! identity, backend/version compatibility, and the calldata encoding that
//! bridges a local proof to an on-chain verifier.
//!
//! # Deliberately not here
//!
//! This crate does **not** make Crucible UltraHonk-specific. It implements
//! the [`ProofProvider`]/[`Verifier`] seams from `crucible-interfaces` the
//! same way `crucible-mock` does; callers only ever see the traits. Actual
//! proof generation against a real Barretenberg binary lands with the
//! circuits batch (see `docs/ultrahonk.md`); today this crate pins the
//! format, encodes calldata, and version-checks artifacts.
//!
//! [`ProofProvider`]: crucible_interfaces::ProofProvider
//! [`Verifier`]: crucible_interfaces::Verifier

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod backend;
pub mod calldata;
pub mod errors;
pub mod proof;
pub mod vk;

pub use backend::{BACKEND_COMPAT, CompatEntry, UltraHonkBackend};
pub use calldata::CalldataEncoder;
pub use errors::UltraHonkError;
pub use proof::{PROOF_FORMAT, PROOF_FORMAT_TAG};
pub use vk::VerificationKeyIdPolicy;
