//! Error type for the proof wire format.

/// Errors produced by the proof wire format.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnvelopeError {
    /// The envelope carries a format version newer than this crate supports.
    #[error("envelope version {found} is newer than the supported version {supported}")]
    UnsupportedVersion {
        /// Version found in the envelope.
        found: u32,
        /// The newest version this crate understands.
        supported: u32,
    },
    /// The envelope could not be (de)serialized.
    #[error("envelope encoding error: {0}")]
    Encoding(String),
}

impl From<serde_json::Error> for EnvelopeError {
    fn from(error: serde_json::Error) -> EnvelopeError {
        EnvelopeError::Encoding(error.to_string())
    }
}
