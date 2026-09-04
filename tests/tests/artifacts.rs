//! Artifact integrity in the live proving path (threat-model A4: the
//! artifact swapper).
//!
//! The provider must refuse to prove against anything but its pinned,
//! manifest-verified artifact. These tests copy the repo's committed
//! `artifacts/circuits/<op>/` into a temp root and attack the copy:
//! single-byte tampering, a missing manifest, a missing bytecode file,
//! and an undeclared extra file must all fail with
//! [`ProviderError::ArtifactIntegrity`] / [`ProviderError::ArtifactUnavailable`]
//! *before* any proving work — no witness is solved, no `bb` invocation
//! happens. Gated on `nargo` + `bb` on PATH, like the other live suites.

use std::path::{Path, PathBuf};

use crucible_interfaces::proof_provider::ProofProvider;
use crucible_interfaces::{BackendId, ProofRequest, ProviderError};
use crucible_noir::NoirToolchain;
use crucible_ultrahonk::{
    BbToolchain, UltraHonkConfig, UltraHonkProvider, VerificationKeyIdPolicy, VkStore,
};
use crucible_vectors::TestVector;
use tempfile::TempDir;

/// The repo's circuits workspace (witness solving runs from source).
fn circuits_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../circuits")
}

/// The repo's committed pinned artifacts.
fn committed_artifacts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../artifacts/circuits")
}

/// Copies the committed pinned artifact for `op` into `dest/<op>/`, returning
/// `dest` — the artifact root the provider expects (it joins `<op>/` itself).
fn copy_pinned(op: &str, dest: &Path) -> PathBuf {
    let from = committed_artifacts().join(op);
    let to = dest.join(op);
    std::fs::create_dir_all(&to).expect("dest dir");
    for name in [String::from("manifest.json"), format!("{op}.json")] {
        std::fs::copy(from.join(&name), to.join(&name)).expect("artifact copied");
    }
    dest.to_path_buf()
}

/// A provider configured to prove from `artifact_root`, with a throwaway VK
/// store. The store dir must outlive the provider.
fn provider_for(artifact_root: &Path) -> (UltraHonkProvider, TempDir) {
    let store_dir = tempfile::tempdir().expect("store temp dir");
    let config = UltraHonkConfig::new(circuits_root(), VkStore::new(store_dir.path()))
        .with_artifact_root(artifact_root);
    (UltraHonkProvider::new(config), store_dir)
}

/// A register request from the committed vector catalog.
fn register_request() -> ProofRequest {
    let vector: TestVector = crucible_vectors::load_catalog(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors"),
    )
    .expect("catalog loads")
    .into_iter()
    .find(|v| v.id == "register-valid-001")
    .expect("register-valid-001 exists");
    vector.to_request_for(BackendId::ULTRAHONK)
}

fn toolchain_ready() -> bool {
    NoirToolchain::is_available() && BbToolchain::is_available()
}

#[test]
fn pinned_artifact_proves_a_register_witness() {
    if !toolchain_ready() {
        eprintln!("skipping: nargo and bb must both be on PATH for the real backend");
        return;
    }
    let work = tempfile::tempdir().expect("temp dir");
    let root = copy_pinned("register", work.path());
    let (provider, _store) = provider_for(&root);
    let request = register_request();
    // Sanity: the helper copied the artifact into `<root>/register/`.
    assert!(root.join("register/manifest.json").exists());

    let response = provider
        .generate(&request)
        .expect("pinned artifact must prove");
    assert_eq!(response.backend.as_str(), BackendId::ULTRAHONK);
    assert_eq!(response.proof.format.as_str(), "ultrahonk-v1");
    let expected = VerificationKeyIdPolicy::id_for(
        &request.circuit,
        &request.circuit_version,
        &response.artifact_checksum,
    );
    assert_eq!(response.verification_key_id.as_str(), expected);
}

#[test]
fn tampered_bytecode_is_rejected_before_proving() {
    if !toolchain_ready() {
        eprintln!("skipping: nargo and bb must both be on PATH for the real backend");
        return;
    }
    let work = tempfile::tempdir().expect("temp dir");
    let root = copy_pinned("register", work.path());
    // Flip one byte of the pinned bytecode after the manifest was written.
    let bytecode = root.join("register/register.json");
    let mut bytes = std::fs::read(&bytecode).expect("bytecode read");
    let last = bytes.len() - 1;
    bytes[last] = if bytes[last] == b'0' { b'1' } else { b'0' };
    std::fs::write(&bytecode, bytes).expect("bytecode written");

    let (provider, _store) = provider_for(&root);
    let err = provider
        .generate(&register_request())
        .expect_err("tampered bytecode must be rejected");
    assert!(
        matches!(err, ProviderError::ArtifactIntegrity { .. }),
        "expected ArtifactIntegrity, got {err}"
    );
}

#[test]
fn missing_manifest_is_an_unavailable_artifact() {
    if !toolchain_ready() {
        eprintln!("skipping: nargo and bb must both be on PATH for the real backend");
        return;
    }
    let work = tempfile::tempdir().expect("temp dir");
    // Bytecode present but no manifest: the strict loader must refuse.
    let op_dir = work.path().join("register");
    std::fs::create_dir_all(&op_dir).expect("dir");
    std::fs::copy(
        committed_artifacts().join("register/register.json"),
        op_dir.join("register.json"),
    )
    .expect("bytecode copied");

    let (provider, _store) = provider_for(work.path());
    let err = provider
        .generate(&register_request())
        .expect_err("missing manifest must be rejected");
    assert!(
        matches!(err, ProviderError::ArtifactUnavailable { .. }),
        "expected ArtifactUnavailable, got {err}"
    );
}

#[test]
fn missing_bytecode_file_is_unavailable() {
    if !toolchain_ready() {
        eprintln!("skipping: nargo and bb must both be on PATH for the real backend");
        return;
    }
    let work = tempfile::tempdir().expect("temp dir");
    let root = copy_pinned("register", work.path());
    std::fs::remove_file(root.join("register/register.json")).expect("bytecode removed");

    let (provider, _store) = provider_for(&root);
    let err = provider
        .generate(&register_request())
        .expect_err("missing bytecode must be rejected");
    assert!(
        matches!(err, ProviderError::ArtifactUnavailable { .. }),
        "expected ArtifactUnavailable, got {err}"
    );
}

#[test]
fn undeclared_extra_file_is_rejected() {
    if !toolchain_ready() {
        eprintln!("skipping: nargo and bb must both be on PATH for the real backend");
        return;
    }
    let work = tempfile::tempdir().expect("temp dir");
    let root = copy_pinned("register", work.path());
    std::fs::write(root.join("register/sneaky.txt"), b"not declared anywhere").expect("extra file");

    let (provider, _store) = provider_for(&root);
    let err = provider
        .generate(&register_request())
        .expect_err("undeclared files must be rejected by the strict loader");
    assert!(
        matches!(err, ProviderError::ArtifactIntegrity { .. }),
        "expected ArtifactIntegrity, got {err}"
    );
}
