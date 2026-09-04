//! Artifact tampering: modified, replaced, or augmented artifacts must be
//! rejected by the loader before any prover or verifier can use them.

use crucible_artifacts::{ArtifactLoader, ArtifactManifest, ManifestFile};
use crucible_interfaces::{ArtifactChecksum, BackendId, CircuitId, Version};
use std::path::Path;

/// Writes a minimal two-file artifact + manifest into `root`.
fn write_artifact(root: &Path, circuit: &CircuitId, version: &Version) {
    std::fs::create_dir_all(root.join("keys")).unwrap();
    std::fs::write(root.join("acir.bin"), b"ACIR-BYTES").unwrap();
    std::fs::write(root.join("keys").join("vk.bin"), b"VK-BYTES").unwrap();
    let manifest = ArtifactManifest {
        manifest_version: crucible_artifacts::manifest::MANIFEST_SCHEMA_VERSION,
        circuit: circuit.clone(),
        circuit_version: *version,
        artifact_version: *version,
        backend: BackendId::new(BackendId::MOCK).unwrap(),
        verification_key_id: None,
        files: vec![
            ManifestFile {
                path: "acir.bin".into(),
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
    std::fs::write(
        root.join(crucible_artifacts::manifest::MANIFEST_FILENAME),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
}

#[test]
fn intact_artifact_loads() {
    let dir = tempfile::tempdir().unwrap();
    write_artifact(
        dir.path(),
        &CircuitId::new("transfer").unwrap(),
        &Version::v0_1(),
    );
    let loaded = ArtifactLoader::new().load(dir.path()).unwrap();
    assert_eq!(loaded.file("acir.bin"), Some(&b"ACIR-BYTES"[..]));
}

#[test]
fn modified_artifact_file_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    write_artifact(
        dir.path(),
        &CircuitId::new("transfer").unwrap(),
        &Version::v0_1(),
    );
    // Swap the ACIR bytes after the manifest was written (attacker replaces
    // the compiled circuit).
    std::fs::write(dir.path().join("acir.bin"), b"EVIL-CIRCUIT").unwrap();
    let err = ArtifactLoader::new().load(dir.path()).unwrap_err();
    assert!(matches!(
        err,
        crucible_artifacts::ArtifactError::ChecksumMismatch { .. }
    ));
}

#[test]
fn planted_extra_file_is_rejected_in_strict_mode() {
    let dir = tempfile::tempdir().unwrap();
    write_artifact(
        dir.path(),
        &CircuitId::new("transfer").unwrap(),
        &Version::v0_1(),
    );
    std::fs::write(dir.path().join("sneaky.bin"), b"payload").unwrap();
    let err = ArtifactLoader::new().load(dir.path()).unwrap_err();
    assert!(matches!(
        err,
        crucible_artifacts::ArtifactError::UnexpectedFile { .. }
    ));
}

#[test]
fn manifest_claiming_a_different_circuit_is_detected_by_pinned_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let circuit = CircuitId::new("transfer").unwrap();
    let version = Version::v0_1();
    write_artifact(dir.path(), &circuit, &version);

    // The loader trusts the manifest's declared identity, so a swapped
    // manifest is only caught by an externally pinned manifest checksum —
    // which is exactly what the manifest_checksum() API enables.
    let manifest = ArtifactManifest::parse(
        &std::fs::read(
            dir.path()
                .join(crucible_artifacts::manifest::MANIFEST_FILENAME),
        )
        .unwrap(),
    )
    .unwrap();
    let expected = manifest.manifest_checksum();

    let mut forged = manifest.clone();
    forged.circuit = CircuitId::new("withdraw").unwrap();
    let json = serde_json::to_vec(&forged).unwrap();
    std::fs::write(
        dir.path()
            .join(crucible_artifacts::manifest::MANIFEST_FILENAME),
        &json,
    )
    .unwrap();

    let parsed = ArtifactManifest::parse(&json).unwrap();
    assert_ne!(
        parsed.manifest_checksum(),
        expected,
        "forged manifest must have a different checksum"
    );
}

#[test]
fn manifest_path_traversal_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("secret.txt");
    std::fs::write(&secret, b"top-secret").unwrap();
    std::fs::write(dir.path().join("acir.bin"), b"ACIR-BYTES").unwrap();

    let manifest = ArtifactManifest {
        manifest_version: crucible_artifacts::manifest::MANIFEST_SCHEMA_VERSION,
        circuit: CircuitId::new("transfer").unwrap(),
        circuit_version: Version::v0_1(),
        artifact_version: Version::v0_1(),
        backend: BackendId::new(BackendId::MOCK).unwrap(),
        verification_key_id: None,
        files: vec![ManifestFile {
            path: "../secret.txt".into(),
            sha256: ArtifactChecksum::from_bytes(b"top-secret"),
            kind: None,
        }],
        backend_metadata: Default::default(),
    };
    let json = serde_json::to_vec(&manifest).unwrap();
    // normalize() alone must reject the traversal path before any file is
    // read from disk.
    assert!(ArtifactManifest::parse(&json).is_err());
}
