//! UltraHonk backend: knowledge and real proving via Barretenberg.
//!
//! UltraHonk is the proving system behind Stellar Confidential Tokens: Noir
//! circuits compiled with `nargo`, proved with the Barretenberg backend, and
//! verified on-chain by an UltraHonk verifier. This crate owns the
//! UltraHonk-specific layer:
//!
//! - [`toolchain`] — locating `bb` and gating its version;
//! - [`exec`] — running `bb prove` / `bb verify` and parsing the
//!   self-describing JSON artifacts (scheme, `bb_version`, `vk_hash`);
//! - [`backend`] — the compatibility matrix pinning the validated
//!   `nargo` × `bb` pairing;
//! - [`proof`] / [`vk`] — proof-format and verification-key identity;
//! - [`calldata`] — encoding public inputs for an on-chain verifier.
//!
//! # Scope boundary
//!
//! This crate does **not** make Crucible UltraHonk-specific. Proving and
//! verification here are *file-level*: they consume a compiled bytecode and
//! a solved witness and produce a proof bundle. Wiring that behind the
//! [`ProofProvider`]/[`Verifier`] seams of `crucible-interfaces` (a
//! verification-key store, the witness bridge from request bags, and the
//! on-chain flow) is the application boundary and lands separately, exactly
//! as `crucible-mock` already implements the traits for tests.
//!
//! [`ProofProvider`]: crucible_interfaces::ProofProvider
//! [`Verifier`]: crucible_interfaces::Verifier

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod backend;
pub mod calldata;
pub mod errors;
pub mod exec;
pub mod proof;
pub mod toolchain;
pub mod vk;

pub use backend::{BACKEND_COMPAT, CompatEntry, UltraHonkBackend};
pub use calldata::CalldataEncoder;
pub use errors::UltraHonkError;
pub use exec::{
    ArtifactFiles, ProofDocument, ProvenArtifacts, ProveOptions, PublicInputsDocument,
    SCHEME_ULTRA_HONK, VerifyOptions, VerifyOutcome, VkDocument, prove, verify,
};
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
