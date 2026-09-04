//! Proving orchestration: the layer between clients and backends.
//!
//! [`crucible-interfaces`] defines *what* a proof request is and *what* a
//! provider answers. This crate implements the *service*: it owns
//!
//! - [`backend::ProviderRegistry`] — which [`ProofProvider`] serves which
//!   backend, and honest dispatch by `(backend, circuit, version)`;
//! - [`prover::ProverService`] — the client-facing [`Prover`] facade that
//!   validates a request, dispatches it, wraps the response in a versioned
//!   [`ProofEnvelope`], and optionally runs the local verification
//!   round-trip;
//! - [`verification`] and [`public_inputs`] — binding checks used by the
//!   round-trip, so a proof that does not match its own public context is
//!   caught immediately;
//! - [`witness`] — the bridge from a request's private/public bags into the
//!   witness model used by encoder and circuits;
//! - [`circuit`] — the canonical circuit catalog and version conventions;
//! - [`artifact`] — deterministic artifact identity (directory, manifest,
//!   checksum policy) shared by tooling;
//! - [`transcript`] — redacted provenance logging of prove/verify runs.
//!
//! The dependency rule holds here too: this crate depends on the *interfaces*
//! and on the mock only in tests. Real backends (Noir, UltraHonk) are
//! registered into a [`ProviderRegistry`](backend::ProviderRegistry) at the
//! application boundary (CLI, adapters), never hard-wired here.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod artifact;
pub mod backend;
pub mod circuit;
pub mod errors;
pub mod proof;
pub mod prover;
pub mod public_inputs;
pub mod transcript;
pub mod verification;
pub mod witness;

pub use backend::ProviderRegistry;
pub use errors::CoreError;
pub use proof::{assemble_envelope, serialized_envelope};
pub use prover::ProverService;
pub use transcript::{TranscriptEntry, TranscriptWriter};
pub use verification::{verify_round_trip, verify_round_trip_with};
