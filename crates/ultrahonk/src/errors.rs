//! Error types for the UltraHonk backend adapter.

/// Errors produced by the UltraHonk backend adapter.
///
/// # Privacy
///
/// Errors carry command names, paths, exit codes, and counts — never witness
/// values and never proof bytes. Toolchain stderr is never echoed verbatim
/// because backend diagnostics may echo source or input material.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UltraHonkError {
    /// A public input did not fit the target encoding.
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

    /// An unsupported circuit/version combination was requested.
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

    /// The `bb` binary could not be found on `PATH`.
    #[error("barretenberg binary not found on PATH; is bb installed? (see scripts/check-bb.sh)")]
    BinaryNotFound,

    /// The installed `bb` version is outside the supported range.
    #[error("unsupported bb version `{found}` (this adapter requires major {supported})")]
    UnsupportedBbVersion {
        /// Version string bb reported.
        found: String,
        /// Minimum supported major version.
        supported: u32,
    },

    /// `bb` reported a version this adapter could not parse.
    #[error("could not parse bb version output: {0}")]
    VersionParse(String),

    /// A required input or output file was missing.
    #[error("missing required file `{path}`")]
    MissingFile {
        /// The missing path.
        path: String,
    },

    /// Reading or writing a file failed.
    #[error("file operation failed on `{path}`: {reason}")]
    Io {
        /// The affected path.
        path: String,
        /// Machine-readable reason.
        reason: String,
    },

    /// The `bb` process could not be spawned.
    #[error("could not run bb: {reason}")]
    Spawn {
        /// Machine-readable reason.
        reason: String,
    },

    /// `bb` exited with a non-zero status on a command that must succeed.
    ///
    /// Semantic rejections (a proof that fails verification) are *outcomes*,
    /// not errors; this variant is for toolchain failures. Only a short
    /// redacted diagnostic excerpt is carried, never raw stderr.
    #[error("bb command `{command}` failed: {reason}")]
    CommandFailed {
        /// The command that failed (e.g. `prove`, `verify`).
        command: String,
        /// Redacted diagnostic excerpt.
        reason: String,
    },

    /// A JSON artifact produced by bb was malformed.
    #[error("bb artifact `{path}` is malformed: {reason}")]
    MalformedArtifact {
        /// The artifact path.
        path: String,
        /// Machine-readable reason.
        reason: String,
    },

    /// Two backend-produced artifacts disagreed with each other (integrity).
    #[error("bb artifacts disagree: {reason}")]
    InconsistentArtifacts {
        /// What disagreed.
        reason: String,
    },
}

impl From<std::io::Error> for UltraHonkError {
    fn from(error: std::io::Error) -> UltraHonkError {
        UltraHonkError::Io {
            path: "<unknown>".to_owned(),
            reason: error.to_string(),
        }
    }
}
