//! Errors produced while loading or verifying circuit artifacts.

use crucible_interfaces::ArtifactChecksum;

/// Every way artifact loading and integrity checking can fail.
///
/// Errors deliberately carry paths and checksums only — never artifact
/// contents (which may include proving-key material).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactError {
    /// The manifest file itself could not be read or parsed.
    #[error("artifact manifest is invalid: {0}")]
    MalformedManifest(String),

    /// The manifest's declared `manifest_version` is not understood by this
    /// crate.
    #[error("manifest version {actual} is not supported by this loader (supported: {supported})")]
    UnsupportedManifestVersion {
        /// Version found in the manifest.
        actual: u32,
        /// Version this loader understands.
        supported: u32,
    },

    /// A file declared in the manifest is missing from the artifact root.
    #[error("artifact file `{path}` is missing from the artifact directory")]
    MissingFile {
        /// Manifest-relative path of the missing file.
        path: String,
    },

    /// A file present in the artifact directory was not declared in the
    /// manifest (only reported in strict mode).
    #[error("unexpected file `{path}` is not declared in the artifact manifest")]
    UnexpectedFile {
        /// Path of the undeclared file.
        path: String,
    },

    /// A file's content does not match the checksum declared in the manifest.
    #[error(
        "artifact file `{path}` failed checksum verification (expected {expected}, computed {actual})"
    )]
    ChecksumMismatch {
        /// Manifest-relative path of the offending file.
        path: String,
        /// Checksum declared in the manifest.
        expected: ArtifactChecksum,
        /// Checksum computed from the bytes on disk.
        actual: ArtifactChecksum,
    },

    /// The artifact tree as a whole does not match the expected checksum.
    #[error("artifact tree failed integrity verification")]
    IntegrityMismatch,

    /// A manifest path escapes the artifact root (path traversal).
    #[error("artifact path `{path}` escapes the artifact directory")]
    UnsafePath {
        /// The offending manifest path.
        path: String,
    },

    /// Reading a file from disk failed.
    #[error("could not read artifact file `{path}`: {reason}")]
    ReadFailure {
        /// The file that could not be read.
        path: String,
        /// Underlying I/O failure description.
        reason: String,
    },
}

impl From<std::io::Error> for ArtifactError {
    fn from(error: std::io::Error) -> ArtifactError {
        ArtifactError::ReadFailure {
            path: String::from("<unknown>"),
            reason: error.to_string(),
        }
    }
}
