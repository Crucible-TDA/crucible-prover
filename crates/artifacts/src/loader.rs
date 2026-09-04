//! The [`ArtifactLoader`]: loads a compiled artifact from disk only after
//! verifying every file against its manifest.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crucible_interfaces::ArtifactChecksum;

use crate::errors::ArtifactError;
use crate::manifest::ArtifactManifest;

/// A successfully loaded artifact: the verified manifest plus the verified
/// contents of every declared file, keyed by manifest path.
///
/// Reaching this type is a guarantee that every declared file existed,
/// contained exactly the bytes its checksum promised, and contained nothing
/// else (in strict mode).
#[derive(Debug, Clone)]
pub struct LoadedArtifact {
    /// The normalized manifest the artifact was loaded against.
    pub manifest: ArtifactManifest,
    /// Verified file contents, keyed by manifest-relative path.
    pub files: Vec<(String, Vec<u8>)>,
}

impl LoadedArtifact {
    /// Returns the verified bytes of one declared file.
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.files
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, bytes)| bytes.as_slice())
    }
}

/// Loads circuit artifacts from a directory, enforcing manifest integrity.
///
/// # Behavior
///
/// 1. Reads `<root>/manifest.json` (or an explicit `manifest` argument) and
///    normalizes it.
/// 2. **Strict mode** (default): rejects any file in the directory that the
///    manifest does not declare — extra files are a sign of a modified
///    artifact directory. Call [`ArtifactLoader::strict`] to relax.
/// 3. Verifies every declared file's bytes against its declared SHA-256
///    before any content is returned.
///
/// Nothing is ever loaded "partially": the first mismatch aborts the whole
/// load and returns a structured [`ArtifactError`].
#[derive(Debug, Clone)]
pub struct ArtifactLoader {
    strict: bool,
}

impl Default for ArtifactLoader {
    fn default() -> ArtifactLoader {
        ArtifactLoader { strict: true }
    }
}

impl ArtifactLoader {
    /// Creates a loader in strict mode (default).
    pub fn new() -> ArtifactLoader {
        ArtifactLoader::default()
    }

    /// Toggles strict mode. In non-strict mode, undeclared files in the
    /// artifact directory are ignored instead of causing rejection.
    pub fn strict(mut self, strict: bool) -> ArtifactLoader {
        self.strict = strict;
        self
    }

    /// Loads and verifies the artifact rooted at `root`, using
    /// `root/manifest.json`.
    pub fn load(&self, root: &Path) -> Result<LoadedArtifact, ArtifactError> {
        let manifest_path = root.join(crate::manifest::MANIFEST_FILENAME);
        let bytes = std::fs::read(&manifest_path).map_err(|e| ArtifactError::ReadFailure {
            path: crate::manifest::MANIFEST_FILENAME.to_owned(),
            reason: e.to_string(),
        })?;
        self.load_with_manifest(root, &bytes)
    }

    /// Loads and verifies the artifact rooted at `root` against an explicit
    /// manifest payload (e.g. one pinned in a release).
    pub fn load_with_manifest(
        &self,
        root: &Path,
        manifest_json: &[u8],
    ) -> Result<LoadedArtifact, ArtifactError> {
        let manifest = ArtifactManifest::parse(manifest_json)?;

        // Strict mode: every on-disk entry (recursively) must be declared.
        if self.strict {
            let declared: BTreeSet<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
            for entry in walk_files(root)? {
                // The manifest itself lives in the artifact directory but is
                // never declared by the manifest.
                if entry == crate::manifest::MANIFEST_FILENAME {
                    continue;
                }
                if !declared.contains(entry.as_str()) {
                    return Err(ArtifactError::UnexpectedFile { path: entry });
                }
            }
        }

        // Verify each declared file before returning anything.
        let mut files = Vec::with_capacity(manifest.files.len());
        for declared in &manifest.files {
            let disk_path = resolve_safe(root, &declared.path)?;
            let bytes = std::fs::read(&disk_path).map_err(|e| ArtifactError::ReadFailure {
                path: declared.path.clone(),
                reason: e.to_string(),
            })?;
            let actual = ArtifactChecksum::from_bytes(&bytes);
            if actual != declared.sha256 {
                return Err(ArtifactError::ChecksumMismatch {
                    path: declared.path.clone(),
                    expected: declared.sha256.clone(),
                    actual,
                });
            }
            files.push((declared.path.clone(), bytes));
        }

        Ok(LoadedArtifact { manifest, files })
    }
}

/// Resolves a manifest path against `root`, rejecting any path that would
/// escape it.
fn resolve_safe(root: &Path, path: &str) -> Result<PathBuf, ArtifactError> {
    if !crate::manifest::is_safe_relative_path(path) {
        return Err(ArtifactError::UnsafePath {
            path: path.to_owned(),
        });
    }
    let joined = root.join(path);
    // Belt and braces: even a technically relative path must stay under root.
    if !joined.starts_with(root) {
        return Err(ArtifactError::UnsafePath {
            path: path.to_owned(),
        });
    }
    Ok(joined)
}

/// Lists every regular file under `root` (recursively) as manifest-style
/// relative paths. Errors on unreadable directories.
fn walk_files(root: &Path) -> Result<Vec<String>, ArtifactError> {
    let mut found = Vec::new();
    collect_files(root, root, &mut found)?;
    Ok(found)
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), ArtifactError> {
    let entries = std::fs::read_dir(dir).map_err(|e| ArtifactError::ReadFailure {
        path: dir.display().to_string(),
        reason: e.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| ArtifactError::ReadFailure {
            path: dir.display().to_string(),
            reason: e.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .expect("walk stays under root")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ArtifactManifest, ManifestFile};
    use crucible_interfaces::{BackendId, CircuitId, Version};

    fn write_artifact(root: &Path) -> ArtifactManifest {
        std::fs::create_dir_all(root.join("keys")).unwrap();
        std::fs::write(root.join("acir.msgpack"), b"ACIR-BYTES").unwrap();
        std::fs::write(root.join("keys").join("vk.bin"), b"VK-BYTES").unwrap();
        let manifest = ArtifactManifest {
            manifest_version: crate::manifest::MANIFEST_SCHEMA_VERSION,
            circuit: CircuitId::new("transfer").unwrap(),
            circuit_version: Version::v0_1(),
            artifact_version: Version::v0_1(),
            backend: BackendId::new(BackendId::ULTRAHONK).unwrap(),
            verification_key_id: None,
            files: vec![
                ManifestFile {
                    path: "acir.msgpack".into(),
                    sha256: ArtifactChecksum::from_bytes(b"ACIR-BYTES"),
                    kind: Some("acir".into()),
                },
                ManifestFile {
                    path: "keys/vk.bin".into(),
                    sha256: ArtifactChecksum::from_bytes(b"VK-BYTES"),
                    kind: Some("verification-key".into()),
                },
            ],
            backend_metadata: Default::default(),
        };
        let json = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(root.join(crate::manifest::MANIFEST_FILENAME), json).unwrap();
        manifest
    }

    #[test]
    fn loads_an_intact_artifact() {
        let dir = tempfile::tempdir().unwrap();
        write_artifact(dir.path());
        let loaded = ArtifactLoader::new().load(dir.path()).unwrap();
        assert_eq!(loaded.file("acir.msgpack"), Some(&b"ACIR-BYTES"[..]));
        assert_eq!(loaded.file("keys/vk.bin"), Some(&b"VK-BYTES"[..]));
        assert_eq!(loaded.manifest.circuit.as_str(), "transfer");
    }

    #[test]
    fn rejects_a_tampered_file() {
        let dir = tempfile::tempdir().unwrap();
        write_artifact(dir.path());
        // Flip one bit in the ACIR file.
        std::fs::write(dir.path().join("acir.msgpack"), b"ACIR-BYTET").unwrap();
        let err = ArtifactLoader::new().load(dir.path()).unwrap_err();
        assert!(
            matches!(err, ArtifactError::ChecksumMismatch { ref path, .. } if path == "acir.msgpack"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        write_artifact(dir.path());
        std::fs::remove_file(dir.path().join("keys").join("vk.bin")).unwrap();
        let err = ArtifactLoader::new().load(dir.path()).unwrap_err();
        assert!(
            matches!(err, ArtifactError::ReadFailure { .. }),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn strict_mode_rejects_extra_files() {
        let dir = tempfile::tempdir().unwrap();
        write_artifact(dir.path());
        std::fs::write(dir.path().join("planted.bin"), b"evil").unwrap();
        let err = ArtifactLoader::new().load(dir.path()).unwrap_err();
        assert!(
            matches!(err, ArtifactError::UnexpectedFile { ref path } if path == "planted.bin"),
            "unexpected error: {err:?}"
        );
        // Non-strict mode ignores the extra file.
        let loaded = ArtifactLoader::new()
            .strict(false)
            .load(dir.path())
            .unwrap();
        assert_eq!(loaded.file("acir.msgpack"), Some(&b"ACIR-BYTES"[..]));
    }

    #[test]
    fn rejects_a_path_traversal_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let secret = dir.path().join("secret.txt");
        std::fs::write(&secret, b"secret").unwrap();
        std::fs::write(dir.path().join("acir.msgpack"), b"ACIR-BYTES").unwrap();

        let mut manifest = write_artifact(dir.path());
        manifest.files.push(ManifestFile {
            path: "../secret.txt".into(),
            sha256: ArtifactChecksum::from_bytes(b"secret"),
            kind: None,
        });
        let json = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(dir.path().join(crate::manifest::MANIFEST_FILENAME), json).unwrap();

        let err = ArtifactLoader::new().load(dir.path()).unwrap_err();
        assert!(matches!(err, ArtifactError::UnsafePath { .. }));
    }

    #[test]
    fn manifest_checksum_catches_manifest_edits() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = write_artifact(dir.path());
        let expected = manifest.manifest_checksum();

        // Reload, then re-write the manifest claiming a different backend.
        let mut manifest = manifest;
        manifest.backend = BackendId::new("evil").unwrap();
        let json = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(dir.path().join(crate::manifest::MANIFEST_FILENAME), &json).unwrap();
        let parsed = ArtifactManifest::parse(&json).unwrap();
        assert_ne!(parsed.manifest_checksum(), expected);
    }
}
