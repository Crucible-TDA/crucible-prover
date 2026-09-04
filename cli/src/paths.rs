//! Repo-relative default paths.
//!
//! The CLI runs from anywhere, so every default resolves relative to the
//! crate manifest (`cli/`), which is always the repo checkout's `cli/`
//! directory: `<repo>/circuits`, `<repo>/test-vectors`, and the
//! verification-key store under `<repo>/artifacts/`.

use std::path::{Path, PathBuf};

/// The repository root (`cli/..`).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Default circuits workspace: `<repo>/circuits`.
pub fn default_circuits_root() -> PathBuf {
    repo_root().join("circuits")
}

/// Default verification-key store: `<repo>/artifacts/verification-keys`.
pub fn default_vk_store() -> PathBuf {
    repo_root().join("artifacts").join("verification-keys")
}

/// The compiled ACIR artifact path for `package` under a circuits workspace.
pub fn artifact_path(circuits: &Path, package: &str) -> PathBuf {
    circuits.join("target").join(format!("{package}.json"))
}