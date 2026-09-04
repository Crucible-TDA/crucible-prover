//! The real UltraHonk backend through the [`ProofProvider`]/[`Verifier`]
//! seams — no mock, no file-level shortcuts.
//!
//! These tests wire `crucible-ultrahonk`'s provider and verifier into the
//! same [`ProverService`] facade the mock uses, so they exercise exactly the
//! contract a simulator or scenario runner would hit: request bags in,
//! verifiable proof responses out, structured rejections for tampered or
//! misattributed submissions. They are gated on `nargo` + `bb` on PATH and
//! on the circuits workspace having compiled bytecode (CI compiles before
//! running), and each test pays for real proving.

use std::path::Path;

use crucible_interfaces::circuit::expectations;
use crucible_interfaces::prover::Prover;
use crucible_interfaces::{
    ArtifactChecksum, BackendId, CircuitId, FieldValue, RootDigest, StateReference,
    VerificationFailure, VerificationKeyId, VerificationRequest, Verifier,
};
use crucible_noir::NoirToolchain;
use crucible_prover_core::ProverService;
use crucible_tests::vectors::{TestVector, load_catalog};
use crucible_ultrahonk::{
    BbToolchain, UltraHonkConfig, UltraHonkProvider, UltraHonkVerifier, VkStore,
    VerificationKeyIdPolicy,
};

/// A fully wired real stack: provider + verifier sharing one VK store.
struct RealStack {
    /// Proving facade with the ultrahonk provider registered.
    service: ProverService,
    /// Verifier sharing the provider's verification-key store.
    verifier: UltraHonkVerifier,
    /// Keeps the store directory alive for the stack's lifetime.
    _store_dir: tempfile::TempDir,
}

/// The circuits workspace, relative to the tests crate manifest.
fn circuits_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../circuits").leak()
}

fn real_stack() -> Option<RealStack> {
    if !NoirToolchain::is_available() || !BbToolchain::is_available() {
        eprintln!("skipping: nargo and bb must both be on PATH for the real backend");
        return None;
    }
    let store_dir = tempfile::tempdir().expect("temp dir");
    let config = UltraHonkConfig::new(circuits_root(), VkStore::new(store_dir.path()));
    let provider = UltraHonkProvider::new(config);
    let verifier = UltraHonkVerifier::new(provider.vk_store().clone());

    let mut service = ProverService::new("tests/0.1.0");
    service.register_provider(Box::new(provider));
    service.with_local_verifier(Box::new(verifier.clone()));
    Some(RealStack {
        service,
        verifier,
        _store_dir: store_dir,
    })
}

fn vector(id: &str) -> TestVector {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors");
    load_catalog(&root)
        .expect("catalog must load")
        .into_iter()
        .find(|v| v.id == id)
        .unwrap_or_else(|| panic!("vector `{id}` must exist in the catalog"))
}

/// A vector's request re-targeted at the real ultrahonk backend.
fn real_request(vector: &TestVector) -> crucible_interfaces::ProofRequest {
    let mut request = vector.to_request();
    request.backend = BackendId::new(BackendId::ULTRAHONK).unwrap();
    request
}

/// Whether the compiled bytecode for `op` exists (CI compiles before this
/// suite runs; local runs need a prior `nargo compile`/`nargo test`).
fn bytecode_present(op: &str) -> bool {
    circuits_root().join("target").join(format!("{op}.json")).is_file()
}

#[test]
fn register_round_trips_through_the_service_with_real_proving() {
    let Some(stack) = real_stack() else {
        return;
    };
    if !bytecode_present("register") {
        eprintln!("skipping: register bytecode not compiled under circuits/target");
        return;
    }
    let vector = vector("register-valid-001");
    let request = real_request(&vector);

    let (response, outcome) = stack
        .service
        .prove_and_verify(&request)
        .expect("prove_and_verify must succeed for a valid witness");
    assert!(outcome.verified, "service round-trip must verify");

    // Response shape: real backend identity and key/artifact provenance.
    assert_eq!(response.backend.as_str(), BackendId::ULTRAHONK);
    assert_eq!(response.proof.format.as_str(), "ultrahonk-v1");
    assert!(!response.proof.bytes.is_empty());
    assert!(response.proof.bytes.len().is_multiple_of(32));
    assert_eq!(response.artifact_checksum.as_hex().len(), 64);
    let (circuit, version, _) =
        VerificationKeyIdPolicy::parse(response.verification_key_id.as_str()).unwrap();
    assert_eq!(circuit, request.circuit);
    assert_eq!(version, request.circuit_version);

    // Public outputs carry the account address the proof commits to.
    let spec = expectations(request.operation);
    assert_eq!(response.public_outputs.len(), spec.public_word_count());
    let address = response.public_outputs.get("account_address").expect("named word");
    let expected = vector.witness.public.get("account_address").unwrap();
    assert_eq!(address, expected, "proof must bind the request's account address");

    // The verification key was persisted under the response's id.
    assert!(stack
        .verifier
        .vk_store()
        .contains(&response.verification_key_id));
}

#[test]
fn transfer_round_trip_names_all_seven_public_words() {
    let Some(stack) = real_stack() else {
        return;
    };
    if !bytecode_present("transfer") {
        eprintln!("skipping: transfer bytecode not compiled under circuits/target");
        return;
    }
    let vector = vector("transfer-valid-001");
    let request = real_request(&vector);

    let (response, outcome) = stack
        .service
        .prove_and_verify(&request)
        .expect("prove_and_verify must succeed for a valid witness");
    assert!(outcome.verified);

    let spec = expectations(request.operation);
    assert_eq!(response.public_outputs.len(), 7, "transfer has seven public words");
    // Public params match the request, returns match the fixture.
    for name in spec.public_params {
        let got = response.public_outputs.get(name).unwrap();
        let want = request.public_inputs.get(name).unwrap();
        assert_eq!(got, want, "public param `{name}` must match the request");
    }
    for name in spec.returns {
        let got = response.public_outputs.get(name).unwrap();
        let want = vector.expected_public_outputs.get(name).unwrap();
        assert_eq!(
            got, want,
            "return `{name}` must match the circuit's reported output"
        );
    }
}

#[test]
fn tampered_proof_is_rejected_with_invalid_proof() {
    let Some(stack) = real_stack() else {
        return;
    };
    if !bytecode_present("register") {
        eprintln!("skipping: register bytecode not compiled");
        return;
    }
    let request = real_request(&vector("register-valid-001"));
    let response = stack
        .service
        .prove(&request)
        .expect("prove must succeed for a valid witness");

    let mut tampered = response.clone();
    let last = tampered.proof.bytes.len() - 1;
    tampered.proof.bytes[last] ^= 0x01;
    let outcome = stack
        .verifier
        .verify(&VerificationRequest::from_response(&tampered))
        .expect("verify must run");
    assert!(
        outcome.rejected_with(VerificationFailure::InvalidProof),
        "a tampered proof must be rejected: {outcome}"
    );
}

#[test]
fn unknown_verification_key_id_is_a_wrong_key_rejection() {
    let Some(stack) = real_stack() else {
        return;
    };
    if !bytecode_present("register") {
        eprintln!("skipping: register bytecode not compiled");
        return;
    }
    let request = real_request(&vector("register-valid-001"));
    let response = stack
        .service
        .prove(&request)
        .expect("prove must succeed for a valid witness");

    // Point the proof at a never-stored key for the same circuit/version and
    // keep the artifact checksum consistent with that id.
    let other_checksum = ArtifactChecksum::from_bytes(b"an artifact we never proved with");
    let id = VerificationKeyId::new(VerificationKeyIdPolicy::id_for(
        &response.circuit,
        &response.circuit_version,
        &other_checksum,
    ))
    .unwrap();
    let mut misattributed = response.clone();
    misattributed.verification_key_id = id;
    misattributed.artifact_checksum = other_checksum;
    let outcome = stack
        .verifier
        .verify(&VerificationRequest::from_response(&misattributed))
        .expect("verify must run");
    assert!(
        outcome.rejected_with(VerificationFailure::WrongVerificationKey),
        "a key the store has never seen must be rejected as wrong: {outcome}"
    );
}

#[test]
fn changed_public_outputs_fail_cryptographic_verification() {
    let Some(stack) = real_stack() else {
        return;
    };
    if !bytecode_present("register") {
        eprintln!("skipping: register bytecode not compiled");
        return;
    }
    let request = real_request(&vector("register-valid-001"));
    let response = stack
        .service
        .prove(&request)
        .expect("prove must succeed for a valid witness");

    // Submit the same proof against a different account address: bb checks
    // the words it commits to and must reject.
    let mut misclaimed = response.clone();
    let original = misclaimed
        .public_outputs
        .get("account_address")
        .unwrap()
        .as_hex()
        .to_owned();
    let flipped = flip_hex_digit(&original);
    misclaimed
        .public_outputs
        .set("account_address", FieldValue::from_hex(&flipped).unwrap())
        .unwrap();
    let outcome = stack
        .verifier
        .verify(&VerificationRequest::from_response(&misclaimed))
        .expect("verify must run");
    assert!(
        !outcome.verified,
        "a proof submitted against different public outputs must be rejected: {outcome}"
    );
}

#[test]
fn circuit_id_mismatch_is_caught_before_cryptography() {
    let Some(stack) = real_stack() else {
        return;
    };
    if !bytecode_present("register") {
        eprintln!("skipping: register bytecode not compiled");
        return;
    }
    let request = real_request(&vector("register-valid-001"));
    let response = stack
        .service
        .prove(&request)
        .expect("prove must succeed for a valid witness");

    // The verification-key id encodes the circuit, so a request claiming a
    // different circuit is rejected structurally before bb runs.
    let mut mismatched = response.clone();
    mismatched.circuit = CircuitId::new("withdraw").unwrap();
    let outcome = stack
        .verifier
        .verify(&VerificationRequest::from_response(&mismatched))
        .expect("verify must run");
    assert!(
        outcome.rejected_with(VerificationFailure::CircuitMismatch),
        "circuit mismatch must be caught before crypto: {outcome}"
    );
}

#[test]
fn state_reference_is_not_yet_cryptographically_bound() {
    // Honest regression pin: the current circuits commit no state root, so a
    // real proof cannot detect that its submission context moved from root A
    // to root B — the mock remains authoritative for stale-state/replay until
    // the circuits fold state roots into their public inputs. Flip this test
    // when they do.
    let Some(stack) = real_stack() else {
        return;
    };
    if !bytecode_present("transfer") {
        eprintln!("skipping: transfer bytecode not compiled");
        return;
    }
    let request = real_request(&vector("transfer-valid-001"));
    let response = stack
        .service
        .prove(&request)
        .expect("prove must succeed for a valid witness");

    let mut stale = response.clone();
    stale.state_reference = Some(StateReference::new(
        RootDigest::from_hex(&"cd".repeat(32)).unwrap(),
        99,
    ));
    let outcome = stack
        .verifier
        .verify(&VerificationRequest::from_response(&stale))
        .expect("verify must run");
    assert!(
        outcome.verified,
        "see the test note: state is repository-level context until circuits bind roots"
    );
}

/// Bumps one hex digit without ever producing a leading zero (so the result
/// stays canonical).
fn flip_hex_digit(hex: &str) -> String {
    let mut chars: Vec<char> = hex.chars().collect();
    let first = chars[0];
    chars[0] = match first {
        'f' => 'e',
        '9' => 'a',
        other => (other as u8 + 1) as char,
    };
    chars.into_iter().collect()
}
