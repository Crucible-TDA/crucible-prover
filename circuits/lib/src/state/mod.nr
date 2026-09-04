//! State-transition helpers: consuming old commitments and producing new
//! ones. The operation circuits compose these into their main().

pub mod transition;

pub use transition::{Consumed, SpendResult};
