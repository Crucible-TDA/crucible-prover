//! Circuit artifact management.
//!
//! Compiled circuits are the trust boundary between "this proof was made by
//! the intended circuit" and "this proof was made by *a* circuit". This crate
//! makes artifact provenance explicit and verifiable:
//!
//! - [`manifest::ArtifactManifest`] declares *which* files make up a compiled
//!   artifact (ACIR, verification key, backend metadata) together with the
//!   exact [`CircuitId`], versions, and backend they belong to.
//! - [`checksum`] computes deterministic SHA-256 fingerprints over files and
//!   over whole artifact trees.
//! - [`loader::ArtifactLoader`] refuses to load anything that does not match
//!   its manifest byte-for-byte: a missing file, an unexpected extra file, or
//!   a single flipped bit all cause a structured rejection *before* the
//!   artifact is handed to a prover or verifier.
//!
//! # Trust model
//!
//! A manifest shipped inside the repository is *not* a root of trust on its
//! own — anyone who can replace the artifact files can also replace the
//! manifest that describes them. What this crate guarantees is **detection**:
//!
//! 1. The [`manifest::ManifestChecksum`] binds the manifest content itself,
//!    so a pinned expected checksum (from CI, a release tag, or a hardware
//!    anchor) detects a tampered manifest.
//! 2. The [`checksum::artifact_checksum`] binds the whole artifact tree, so a
//!    caller that holds the manifest's own checksum detects file tampering
//!    even if individual file hashes were edited in place.
//!
//! See `docs/artifacts.md` for the full policy.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod checksum;
pub mod errors;
pub mod loader;
pub mod manifest;

pub use errors::ArtifactError;
pub use loader::ArtifactLoader;
pub use manifest::{ArtifactManifest, MANIFEST_SCHEMA_VERSION, ManifestFile};
