//! Error type for UltraHonk backend operations.

/// Errors produced while encoding calldata or resolving UltraHonk identity.
///
/// # Privacy
///
/// Errors carry names and counts, never values. Encoding failures never echo
/// the public-input bytes that failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UltraHonkError {
    /// A field value did not fit the target encoding.
    #[error("public input `{name}` could not be encoded: {reason}")]
    Encode {
        /// Name of the offending public input.
        name: String,
        /// Machine-readable reason.
        reason: String,
    },

    /// Calldata had fewer public inputs than the header declared.
    #[error("calldata declares {expected} public inputs but carries {actual}")]
    Truncated {
        /// Declared count.
        expected: usize,
        /// Actual count.
        actual: usize,
    },

    /// The verification key id does not follow the expected scheme.
    #[error("verification key id `{id}` does not follow the expected scheme")]
    BadVerificationKeyId {
        /// The offending id.
        id: String,
    },

    /// An unsupported backend/version combination was requested.
    #[error(
        "unsupported combination: backend `{backend}` version {version} (supported: {supported})"
    )]
    UnsupportedVersion {
        /// Backend name.
        backend: String,
        /// Requested version.
        version: String,
        /// Comma-joined supported versions.
        supported: String,
    },
}
