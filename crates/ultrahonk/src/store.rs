//! Verification-key storage.
//!
//! A [`VerificationRequest`] carries a [`VerificationKeyId`] but not the key
//! itself — the verifier must resolve key bytes from that id. This module is
//! the filesystem-backed resolution layer for the UltraHonk backend: the
//! provider writes the `vk.json` a proof was produced with, and the verifier
//! reads it back by id.
//!
//! # Id layout
//!
//! Files live under `<root>/<circuit>/<version>/<artifact-hash>.json`, so a
//! store directory is self-describing and trivially inspectable. Ids that do
//! not follow the `uhk/…` scheme (e.g. mock ids in tests that share a
//! directory) fall back to a hashed path and will simply be absent.

use std::path::{Path, PathBuf};

use crucible_interfaces::VerificationKeyId;

use crate::errors::UltraHonkError;
use crate::exec::{SCHEME_ULTRA_HONK, VkDocument};
use crate::vk::VerificationKeyIdPolicy;

/// A directory of verification keys, keyed by [`VerificationKeyId`].
#[derive(Debug, Clone)]
pub struct VkStore {
    root: PathBuf,
}

impl VkStore {
    /// Opens a store rooted at `root` (created on demand).
    pub fn new(root: impl Into<PathBuf>) -> VkStore {
        VkStore { root: root.into() }
    }

    /// The store root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The on-disk path a verification key id maps to.
    ///
    /// `uhk/…` ids map to `<root>/<circuit>/<version>/<artifact-hash>.json`;
    /// any other id maps to a hashed path so foreign ids never collide or
    /// escape the root.
    pub fn path_for(&self, id: &VerificationKeyId) -> PathBuf {
        match VerificationKeyIdPolicy::parse(id.as_str()) {
            Ok((circuit, version, artifact)) => self
                .root
                .join(circuit.as_str())
                .join(version.to_string())
                .join(format!("{}.json", artifact.as_hex())),
            Err(_) => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(id.as_str().as_bytes());
                self.root
                    .join("_foreign")
                    .join(format!("{}.json", hex::encode(hasher.finalize())))
            }
        }
    }

    /// Stores a verification key under `id`.
    ///
    /// The stored document must be a well-formed UltraHonk VK; the digest it
    /// carries is preserved so the verifier can cross-check it.
    pub fn put(&self, id: &VerificationKeyId, doc: &VkDocument) -> Result<(), UltraHonkError> {
        if doc.scheme != SCHEME_ULTRA_HONK {
            return Err(UltraHonkError::InconsistentArtifacts {
                reason: format!(
                    "refusing to store a VK with scheme `{}` under an UltraHonk id",
                    doc.scheme
                ),
            });
        }
        let path = self.path_for(id);
        let parent = path.parent().ok_or_else(|| UltraHonkError::Io {
            path: path.display().to_string(),
            reason: "no parent directory".to_owned(),
        })?;
        std::fs::create_dir_all(parent).map_err(|e| UltraHonkError::Io {
            path: parent.display().to_string(),
            reason: e.to_string(),
        })?;
        let json = serde_json::to_string(doc).map_err(|e| UltraHonkError::Io {
            path: path.display().to_string(),
            reason: format!("cannot serialize verification key: {e}"),
        })?;
        std::fs::write(&path, json).map_err(|e| UltraHonkError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Loads the verification key stored under `id`.
    ///
    /// A missing key is reported as [`UltraHonkError::MissingFile`] so
    /// callers can map it to a "wrong verification key" verdict rather than a
    /// toolchain error.
    pub fn get(&self, id: &VerificationKeyId) -> Result<VkDocument, UltraHonkError> {
        let path = self.path_for(id);
        if !path.is_file() {
            return Err(UltraHonkError::MissingFile {
                path: path.display().to_string(),
            });
        }
        let text = std::fs::read_to_string(&path).map_err(|e| UltraHonkError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        let doc: VkDocument =
            serde_json::from_str(&text).map_err(|e| UltraHonkError::MalformedArtifact {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?;
        if doc.scheme != SCHEME_ULTRA_HONK {
            return Err(UltraHonkError::MalformedArtifact {
                path: path.display().to_string(),
                reason: format!("stored VK has scheme `{}`", doc.scheme),
            });
        }
        Ok(doc)
    }

    /// Whether a verification key exists under `id`.
    pub fn contains(&self, id: &VerificationKeyId) -> bool {
        self.path_for(id).is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec::VkDocument;
    use tempfile::tempdir;

    fn sample_id() -> VerificationKeyId {
        let circuit = crucible_interfaces::CircuitId::new("transfer").unwrap();
        let version = crucible_interfaces::Version::new(0, 1, 0);
        let hash = crucible_interfaces::ArtifactChecksum::from_hex(&"ab".repeat(32)).unwrap();
        VerificationKeyId::new(VerificationKeyIdPolicy::id_for(&circuit, &version, &hash)).unwrap()
    }

    fn sample_vk() -> VkDocument {
        VkDocument {
            vk: vec!["0x00".into(), "0x0e".into()],
            hash: "0x12804588d2137c4293a920afbd63c968d8e847a0cf59704e58440ea0fb7d5cf9".to_owned(),
            bb_version: "6.0.0-nightly.20260903".to_owned(),
            scheme: SCHEME_ULTRA_HONK.to_owned(),
        }
    }

    #[test]
    fn put_and_get_round_trip_by_id() {
        let dir = tempdir().unwrap();
        let store = VkStore::new(dir.path());
        let id = sample_id();
        assert!(!store.contains(&id));
        store.put(&id, &sample_vk()).unwrap();
        assert!(store.contains(&id));
        let loaded = store.get(&id).unwrap();
        assert_eq!(loaded, sample_vk());
        assert_eq!(
            store.path_for(&id),
            dir.path()
                .join("transfer/0.1.0")
                .join(format!("{}.json", "ab".repeat(32)))
        );
    }

    #[test]
    fn missing_key_is_reported_as_missing_file() {
        let dir = tempdir().unwrap();
        let store = VkStore::new(dir.path());
        let err = store.get(&sample_id()).unwrap_err();
        assert!(matches!(err, UltraHonkError::MissingFile { .. }));
    }

    #[test]
    fn foreign_ids_map_to_hashed_paths() {
        let dir = tempdir().unwrap();
        let store = VkStore::new(dir.path());
        let mock_id = VerificationKeyId::new("mock-vk/transfer/0.1.0").unwrap();
        let path = store.path_for(&mock_id);
        assert!(path.starts_with(dir.path().join("_foreign")));
        assert!(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".json"),
            "foreign id path must end in .json: {}",
            path.display()
        );
    }

    #[test]
    fn foreign_scheme_documents_are_refused() {
        let dir = tempdir().unwrap();
        let store = VkStore::new(dir.path());
        let mut doc = sample_vk();
        doc.scheme = "chonk".to_owned();
        let err = store.put(&sample_id(), &doc).unwrap_err();
        assert!(matches!(err, UltraHonkError::InconsistentArtifacts { .. }));
    }
}
