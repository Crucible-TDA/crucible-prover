//! Shared cryptographic and constraint primitives for the Crucible circuits.
//!
//! Every operation circuit (`register`, `deposit`, `merge`, `transfer`,
//! `withdraw`) imports this library so the primitives they share — types,
//! commitment construction, hashing, nullifiers, range and ownership
//! constraints — are defined once and measured once in the gadgets.
//!
//! # Scope honesty
//!
//! These primitives implement the *shape* of the Confidential Token
//! semantics with Pedersen commitments and range-bounded amounts so the
//! proving architecture can be exercised. The exact scheme (curve, hash
//! domain separation, key derivation, commitment layout) must be aligned
//! with the target Confidential Token circuit specification before
//! production use; see `docs/noir.md`.

pub mod constraints;
pub mod crypto;
pub mod state;
pub mod types;

pub use constraints::range;
pub use crypto::commitments;
pub use crypto::hashing;
pub use state::transition;
pub use types::{account, commitment, token};

/// Upper bound on confidential amounts: values must fit in this many bits so
/// range constraints are cheap and sums cannot wrap the field modulus.
pub global AMOUNT_BITS: u32 = 64;
