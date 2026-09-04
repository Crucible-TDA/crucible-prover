//! TEST ONLY — deterministic mock prover and verifier.
//!
//! # NOT CRYPTOGRAPHICALLY SECURE
//!
//! The mock backend performs **no cryptography**. Its "proofs" are
//! self-describing, deterministic envelopes that bind every public-context
//! field (circuit, version, backend, verification key id, artifact checksum,
//! public outputs, state reference) so that the full prover/verifier
//! contract — request validation, envelope assembly, context binding,
//! rejection of tampered or misattributed proofs — can be exercised in CI
//! without paying for real proving.
//!
//! A mock proof can be forged by anyone who reads this source. Never use the
//! mock backend outside tests, fuzzing harnesses, and scenario runs that
//! explicitly trade cryptographic soundness for speed.
//!
//! # What the mock is for
//!
//! - Unit and integration tests of `prover-core`, the verifier service, and
//!   the security suite (tamper, replay, stale-state, wrong-key).
//! - Fast scenario execution in `crucible-scenarios`.
//! - Deterministic fixtures: the same [`ProofRequest`] always produces the
//!   same proof bytes, so golden files are stable across runs.
//!
//! # What the mock is not for
//!
//! - Any test whose *point* is cryptographic soundness.
//! - Measuring constraint counts, proof sizes, or proving time.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod fixtures;
pub mod prover;
pub mod verifier;

pub use prover::MockProver;
pub use verifier::MockVerifier;
