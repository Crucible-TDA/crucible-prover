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
pub mod toolchain;
pub mod vk;

pub use backend::{BACKEND_COMPAT, CompatEntry, UltraHonkBackend};
pub use calldata::CalldataEncoder;
pub use errors::UltraHonkError;
pub use proof::{PROOF_FORMAT, PROOF_FORMAT_TAG};
pub use toolchain::{BbToolchain, BbVersion};
pub use vk::VerificationKeyIdPolicy;

/// The default command used to locate `bb`.
pub const BB_BIN: &str = "bb";

/// The minimum `bb` major version this adapter understands.
///
/// The major floor excludes pre-2026 Barretenberg CLI generations whose
/// command surface and artifact formats differ from what this adapter is
/// developed against; the exact validated pairing is pinned in
/// [`BACKEND_COMPAT`].
pub const MIN_BB_MAJOR: u32 = 4;

/// The `bb` version this adapter is validated against (see [`BACKEND_COMPAT`]).
pub const TESTED_BB_VERSION: &str = "6.0.0-nightly.20260903";
