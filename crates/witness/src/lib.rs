//! Private witness management.
//!
//! The witness is where private information enters the proving system, so
//! this crate is deliberately strict about the boundary between
//!
//! ```text
//! PRIVATE WITNESS   (secret values, never logged/serialized by accident)
//! PUBLIC INPUT      (bound context, safe to log and fixture)
//! ```
//!
//! Responsibilities:
//!
//! - [`builder`] — assemble [`WitnessData`](builder::WitnessData) from a
//!   request or from explicit public/private bags, enforcing that the two
//!   sides never overlap and that required names are present.
//! - [`encoder`] — encode a witness into the Noir `Prover.toml` layout and
//!   write it to disk with restrictive permissions. This is the *single*
//!   intentional place where secret values leave memory.
//! - [`decoder`] — parse public outputs back into
//!   [`crucible_interfaces::PublicInputBag`]s. The decoder never
//!   reconstructs private values.
//! - [`validation`] — structural validation of assembled witnesses.
//! - [`secrets`] — redaction helpers so long hex material (which is what
//!   secrets look like) is scrubbed from logs, errors, and CLI output.
//!
//! # Leakage rule
//!
//! Nothing in this crate prints a secret value. `WitnessData` and the private
//! bag implement `Debug` as redacted views (names only); the encoder is the
//! sole escape hatch and its output must never be logged — only written to a
//! file with `0600` permissions (Unix) or handed straight to the prover.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod builder;
pub mod decoder;
pub mod encoder;
pub mod errors;
pub mod secrets;
pub mod validation;

pub use builder::{WitnessAssembler, WitnessData};
pub use errors::WitnessError;
