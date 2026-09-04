//! Assertion-message vocabulary.
//!
//! Noir has no error enums; constraint failures are `assert` messages whose
//! literal text differs in length (Noir `str<N>` types are length-exact, so
//! they cannot share one global type). The failure classes below are
//! therefore a *vocabulary*: every circuit writes assertions with exactly
//! these messages, and tooling (the Rust adapter's error mapping, test
//! vectors) matches on them.
//!
//! - **commitment does not open to the provided witness** — a supplied
//!   opening `(amount, blinding)` did not match the public commitment.
//! - **value exceeds the allowed range** — an amount exceeded the
//!   range bound.
//! - **insufficient balance for the operation** — a spend exceeded the
//!   balance its opening showed.
//! - **account secret does not match the address** — the wrong owner secret
//!   was supplied.
//! - **token mismatch** — an operation mixed tokens.
//! - **commitment has already been consumed** — a transition referenced
//!   already-spent state.
//!
//! # Privacy
//!
//! Assertion messages are constant strings and never embed witness values.
