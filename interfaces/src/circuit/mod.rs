//! Circuit identities, versions, operations, and the value model.
//!
//! Everything a proof is *about* is described with these types: which circuit
//! produced it ([`CircuitId`]), which version of that circuit
//! ([`Version`]), which protocol operation it implements ([`Operation`]),
//! and which values flow into the witness ([`PublicInputBag`],
//! [`PrivateWitnessBag`]).

// The submodules mirror the repository plan's `circuit/` folder exactly, so
// `circuit.rs` lives inside the `circuit` module. clippy's module_inception
// lint is not useful here; the nesting is intentional.
#[allow(clippy::module_inception)]
mod circuit;
mod inputs;
mod metadata;
mod outputs;

pub use circuit::{CircuitId, CircuitIdError, Operation, Version};
pub use inputs::{FieldError, FieldValue, PrivateWitnessBag, PublicInputBag, SecretValue};
pub use metadata::CircuitMetadata;
pub use outputs::OutputBag;
