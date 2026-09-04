//! Error type for witness management.

use crucible_interfaces::FieldError;
use crucible_interfaces::Operation;

/// Which side of the witness a problem refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessSide {
    /// A public (bound) input.
    Public,
    /// A private (secret) witness value.
    Private,
}

/// Errors produced while assembling, validating, encoding, or decoding
/// witnesses.
///
/// # Privacy
///
/// Error messages carry names and operations only — never values. If an
/// underlying I/O or encoding failure message could contain witness material,
/// it must be classified before being wrapped.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WitnessError {
    /// A required witness name was missing.
    #[error("operation {operation} requires {side:?} witness `{name}`")]
    MissingRequired {
        /// The operation being assembled.
        operation: Operation,
        /// Which side was missing the value.
        side: WitnessSide,
        /// The missing name.
        name: String,
    },

    /// The same name appeared on both the public and the private side.
    #[error("witness name `{name}` appears on both the public and private side")]
    Overlap {
        /// The overlapping name.
        name: String,
    },

    /// The operation does not match the request being assembled from.
    #[error("operation mismatch: assembler expects {expected}, request is {actual}")]
    OperationMismatch {
        /// Operation the assembler was built for.
        expected: Operation,
        /// Operation the request carried.
        actual: Operation,
    },

    /// A value failed validation.
    #[error(transparent)]
    InvalidValue(#[from] FieldError),

    /// Witness file I/O failed.
    #[error("witness I/O error: {0}")]
    Io(String),

    /// Encoding or decoding failed.
    #[error("witness encoding error: {0}")]
    Encoding(String),
}

impl From<std::io::Error> for WitnessError {
    fn from(error: std::io::Error) -> WitnessError {
        WitnessError::Io(error.to_string())
    }
}
