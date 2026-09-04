//! Error types for the Noir toolchain adapter.

/// Errors produced while interacting with the Noir toolchain.
///
/// # Privacy
///
/// Error messages carry command names and status codes only. Toolchain
/// stderr is never echoed verbatim into errors because compiler diagnostics
/// may echo source snippets that contain witness values.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NoirError {
    /// The `nargo` binary could not be found on `PATH`.
    #[error("nargo binary not found on PATH; is the Noir toolchain installed?")]
    BinaryNotFound,

    /// The installed `nargo` version is outside the supported range.
    #[error("unsupported nargo version `{found}` (this adapter supports major {supported})")]
    UnsupportedVersion {
        /// Version string nargo reported.
        found: String,
        /// Supported major version.
        supported: u32,
    },

    /// `nargo` reported a version this adapter could not parse.
    #[error("could not parse nargo version output: {0}")]
    VersionParse(String),

    /// `nargo` exited with a non-zero status.
    #[error("nargo command `{command}` failed with exit status {status}")]
    CommandFailed {
        /// The command that failed.
        command: String,
        /// The exit status.
        status: i32,
    },

    /// A required file was missing after a nargo run.
    #[error("expected `{path}` to exist after `nargo {command}`, but it does not")]
    ExpectedOutput {
        /// The expected path.
        path: String,
        /// The nargo command that should have produced it.
        command: String,
    },

    /// Reading a produced artifact from disk failed.
    #[error("could not read `{path}`: {reason}")]
    Io {
        /// The unreadable path.
        path: String,
        /// Machine-readable reason.
        reason: String,
    },

    /// A compiled artifact JSON was malformed.
    #[error("artifact `{path}` is malformed: {reason}")]
    MalformedArtifact {
        /// The artifact path.
        path: String,
        /// Machine-readable reason.
        reason: String,
    },
}

impl From<std::io::Error> for NoirError {
    fn from(error: std::io::Error) -> NoirError {
        NoirError::Io {
            path: "<unknown>".to_owned(),
            reason: error.to_string(),
        }
    }
}
