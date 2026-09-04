//! Deterministic checksum helpers for artifact files and trees.

use std::path::Path;

use crucible_interfaces::ArtifactChecksum;

/// Computes the SHA-256 digest of a file's bytes.
pub fn file_checksum(path: &Path) -> std::io::Result<ArtifactChecksum> {
    let bytes = std::fs::read(path)?;
    Ok(ArtifactChecksum::from_bytes(&bytes))
}

/// Computes the whole-tree artifact checksum from a list of
/// `(relative_path, checksum)` pairs.
///
/// The input is the manifest's `files` list (sorted by the manifest's
/// normalization), so the result is deterministic for a given artifact.
/// The binding covers *paths* as well as contents: renaming a file inside
/// the artifact changes the digest even if the bytes are identical.
pub fn artifact_checksum(entries: &[(String, ArtifactChecksum)]) -> ArtifactChecksum {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    for (path, checksum) in entries {
        hasher.update(path.as_bytes());
        hasher.update([0u8]);
        hasher.update(checksum.as_hex().as_bytes());
        hasher.update([0u8]);
    }
    ArtifactChecksum::from_bytes(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_checksum_is_deterministic_and_path_sensitive() {
        let a = ArtifactChecksum::from_hex(&"aa".repeat(32)).unwrap();
        let b = ArtifactChecksum::from_hex(&"bb".repeat(32)).unwrap();
        let entries = vec![
            ("acir.msgpack".to_owned(), a.clone()),
            ("vk.bin".to_owned(), b.clone()),
        ];
        let first = artifact_checksum(&entries);
        assert_eq!(first, artifact_checksum(&entries));

        // Same content under a different path must differ.
        let renamed = vec![
            ("acir.msgpack".to_owned(), a.clone()),
            ("other/vk.bin".to_owned(), b),
        ];
        assert_ne!(first, artifact_checksum(&renamed));
    }

    #[test]
    fn file_checksum_matches_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blob.bin");
        std::fs::write(&path, b"artifact bytes").unwrap();
        assert_eq!(
            file_checksum(&path).unwrap(),
            ArtifactChecksum::from_bytes(b"artifact bytes")
        );
    }
}
