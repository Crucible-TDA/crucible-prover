//! Adapter for the Noir toolchain (`nargo`).
//!
//! All interaction with the `nargo` binary lives behind this crate so that
//! process execution is never scattered through the codebase. The adapter
//! owns:
//!
//! - [`toolchain`] — locating the `nargo` binary and verifying its version
//!   against the range this crate supports;
//! - [`compiler`] — running `nargo compile` (ACIR) and `nargo execute`
//!   (witness generation) on a circuit project;
//! - [`artifact`] — reading and validating the compiled artifact JSON that
//!   `nargo compile` produces in `target/`;
//! - [`info`] — parsing `nargo info` output for constraint/circuit metrics.
//!
//! # Scope boundary
//!
//! `nargo` compiles circuits and solves witnesses; it does **not** generate
//! or verify UltraHonk proofs in current toolchains (proving moved to the
//! Barretenberg backend). Proof generation/verification therefore live in the
//! `ultrahonk` adapter, which consumes this crate's artifacts and witnesses.
//!
//! # Privacy rule
//!
//! Witness values are written to `Prover.toml` only through
//! [`crucible-witness`]'s encoder (which enforces restrictive permissions)
//! and are never echoed into this crate's errors or logs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod artifact;
pub mod compiler;
pub mod errors;
pub mod info;
pub mod toolchain;

pub use errors::NoirError;
pub use toolchain::NoirToolchain;

/// The default command used to locate `nargo`.
pub const NARGO_BIN: &str = "nargo";

/// The minimum `nargo` major version this adapter understands.
pub const MIN_NARGO_MAJOR: u32 = 1;

/// The version of `nargo` this adapter is tested against.
pub const TESTED_NARGO_VERSION: &str = "1.0.0-beta.26";
