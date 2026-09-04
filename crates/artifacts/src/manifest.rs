//! The [`ArtifactManifest`]: a versioned, JSON-serializable description of a
//! compiled circuit artifact.

use std::collections::BTreeMap;

use crucible_interfaces::{ArtifactChecksum, BackendId, CircuitId, VerificationKeyId, Version};
use serde::{Deserialize, Serialize};

/// The version of the manifest schema this crate reads and writes.
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Canonical filename a manifest is expected to have inside an artifact
/// directory.
pub const MANIFEST_FILENAME: &str = "manifest.json";

/// One file that belongs to a compiled artifact.
///
/// `path` is relative to the artifact root and must stay inside it (the
/// loader enforces this — see [`crate::loader`]). `sha256` is the digest of
/// the file's exact bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFile {
    /// Artifact-relative path, always using `/` separators.
    pub path: String,
    /// SHA-256 digest of the file bytes.
    pub sha256: ArtifactChecksum,
    /// Optional role description (e.g. `acir`, `verification-key`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// A complete, self-describing description of one compiled circuit artifact.
///
/// The manifest answers the four provenance questions of any proof produced
/// from the artifact:
///
/// - **which circuit** — [`ArtifactManifest::circuit`] and `circuit_version`;
/// - **which artifact generation** — `artifact_version`;
/// - **which backend** — [`ArtifactManifest::backend`] plus free-form
///   `backend_metadata` (e.g. the Barretenberg commit a key was generated
///   with);
/// - **which verification key** — `verification_key_id`, when the artifact
///   embeds or implies one.
///
/// The manifest is JSON-serializable and its [`ManifestFile`] list is ordered
/// deterministically (sorted by path), so the same artifact always produces
/// byte-identical JSON — a prerequisite for stable checksums.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactManifest {
    /// Schema version of this manifest.
    pub manifest_version: u32,
    /// The circuit this artifact was compiled from.
    pub circuit: CircuitId,
    /// Version of the circuit source.
    pub circuit_version: Version,
    /// Generation of the compiled artifact (bumps when the artifact is
    /// recompiled without a circuit change, e.g. a backend upgrade).
    pub artifact_version: Version,
    /// The backend the artifact targets.
    pub backend: BackendId,
    /// Verification key id associated with this artifact, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_key_id: Option<VerificationKeyId>,
    /// The files that make up the artifact, sorted by `path`.
    pub files: Vec<ManifestFile>,
    /// Free-form backend/compiler metadata (never interpreted by the loader).
    #[serde(default)]
    pub backend_metadata: BTreeMap<String, String>,
}

impl ArtifactManifest {
    /// Validates invariants that cannot be expressed in the type system and
    /// sorts `files` deterministically.
    ///
    /// Called by [`ArtifactManifest::parse`] and re-run by the loader before
    /// any file is touched, so a hand-edited manifest is normalized too.
    pub fn normalize(mut self) -> Result<ArtifactManifest, crate::ArtifactError> {
        if self.manifest_version != MANIFEST_SCHEMA_VERSION {
            return Err(crate::ArtifactError::UnsupportedManifestVersion {
                actual: self.manifest_version,
                supported: MANIFEST_SCHEMA_VERSION,
            });
        }
        for file in &self.files {
            if !is_safe_relative_path(&file.path) {
                return Err(crate::ArtifactError::UnsafePath {
                    path: file.path.clone(),
                });
            }
        }
        self.files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(self)
    }

    /// Parses manifest JSON and normalizes it.
    pub fn parse(json: &[u8]) -> Result<ArtifactManifest, crate::ArtifactError> {
        let manifest: ArtifactManifest = serde_json::from_slice(json)
            .map_err(|e| crate::ArtifactError::MalformedManifest(e.to_string()))?;
        manifest.normalize()
    }

    /// Returns the manifest in canonical JSON form (compact, sorted keys).
    pub fn to_canonical_json(&self) -> Vec<u8> {
        // serde_json::Value::to_string on a struct serializes fields in
        // declaration order, which is deterministic for this type; the
        // `files` vec is sorted by normalize() and `backend_metadata` is a
        // BTreeMap, so output is stable.
        serde_json::to_vec(self).expect("manifest serialization cannot fail")
    }

    /// Computes the checksum that binds this manifest's *declared* content.
    ///
    /// Pinning this value externally (CI, release tag) detects a tampered
    /// manifest, which file-level checks alone cannot.
    pub fn manifest_checksum(&self) -> ArtifactChecksum {
        ArtifactChecksum::from_bytes(&self.to_canonical_json())
    }

    /// Looks up the declared checksum of `path`, if declared.
    pub fn file_checksum(&self, path: &str) -> Option<&ArtifactChecksum> {
        self.files
            .iter()
            .find(|f| f.path == path)
            .map(|f| &f.sha256)
    }
}

/// Whether a manifest path is safe to resolve against an artifact root.
///
/// Accepts relative, `/`-separated paths that do not begin with `/`, do not
/// contain `..` components, and do not contain NUL bytes. This is what keeps
/// a malicious manifest from reading arbitrary files off disk.
pub fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        // Backslash is a path separator on Windows; manifest paths always
        // use '/', so rejecting it keeps manifests platform-independent.
        && !path.contains('\\')
        && !path.bytes().any(|b| b == 0)
        && path.split('/').all(|component| {
            !component.is_empty() && component != "." && component != ".."
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ArtifactManifest {
        let circuit = CircuitId::new("transfer").unwrap();
        let files = vec![
            ManifestFile {
                path: "acir.msgpack".into(),
                sha256: ArtifactChecksum::from_hex(&"aa".repeat(32)).unwrap(),
                kind: Some("acir".into()),
            },
            ManifestFile {
                path: "vk.bin".into(),
                sha256: ArtifactChecksum::from_hex(&"bb".repeat(32)).unwrap(),
                kind: Some("verification-key".into()),
            },
        ];
        ArtifactManifest {
            manifest_version: MANIFEST_SCHEMA_VERSION,
            circuit,
            circuit_version: Version::v0_1(),
            artifact_version: Version::v0_1(),
            backend: BackendId::new(BackendId::ULTRAHONK).unwrap(),
            verification_key_id: Some(VerificationKeyId::new("vk-transfer-0.1.0").unwrap()),
            files,
            backend_metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn json_round_trip_is_stable() {
        let manifest = sample_manifest().normalize().unwrap();
        let json = manifest.to_canonical_json();
        let parsed = ArtifactManifest::parse(&json).unwrap();
        assert_eq!(parsed, manifest);
        // Serializing twice must produce identical bytes.
        assert_eq!(parsed.to_canonical_json(), json);
    }

    #[test]
    fn normalize_sorts_files() {
        let mut manifest = sample_manifest();
        manifest.files.reverse();
        let manifest = manifest.normalize().unwrap();
        let paths: Vec<&str> = manifest.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["acir.msgpack", "vk.bin"]);
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        for bad in [
            "",
            "/etc/passwd",
            "..",
            "../outside",
            "a/../../outside",
            "a/./b",
            "a//b",
            "a\\b",
        ] {
            assert!(
                !is_safe_relative_path(bad),
                "path should be unsafe: {bad:?}"
            );
        }
        for good in ["acir.msgpack", "keys/vk.bin", "a-b_c.d"] {
            assert!(is_safe_relative_path(good), "path should be safe: {good:?}");
        }
    }

    #[test]
    fn unsupported_manifest_version_is_rejected() {
        let mut manifest = sample_manifest();
        manifest.manifest_version = 999;
        assert!(matches!(
            manifest.normalize(),
            Err(crate::ArtifactError::UnsupportedManifestVersion { .. })
        ));
    }

    #[test]
    fn checksum_binds_manifest_content() {
        let a = sample_manifest().normalize().unwrap();
        let mut b = a.clone();
        b.artifact_version = Version::new(0, 2, 0);
        assert_ne!(a.manifest_checksum(), b.manifest_checksum());
    }
}
