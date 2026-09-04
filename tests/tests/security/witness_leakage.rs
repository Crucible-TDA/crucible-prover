//! Witness leakage: private values must never appear in any observable
//! output — debug views, errors, transcripts, or serialized documents.

use crucible_interfaces::{ProofProvider, Verifier};
use crucible_mock::MockProver;

/// Distinctive secret hex values planted in the transfer fixture.
///
/// Markers are long (≥10 hex chars) on purpose: short values like `7f` could
/// legitimately appear inside a SHA-256 checksum by chance and make these
/// tests flaky, whereas a 10+ hex-char marker colliding with a digest is
/// astronomically unlikely.
const KNOWN_SECRETS: [&str; 2] = ["deadbeefcafe", "0102030405"];

/// A deterministic way to prove a specific private value would be detected
/// if it leaked: insert it, leak it into a string, and check the scan.
#[test]
fn the_leak_scanner_itself_detects_planted_secrets() {
    let planted = format!("a {} b", KNOWN_SECRETS[0]);
    assert!(planted.contains(KNOWN_SECRETS[0]));
    // Sanity: the marker does not appear in public context by chance.
    let request = super::fixtures::transfer_request();
    let public_json = serde_json::to_string(&request.public_inputs).unwrap();
    for secret in KNOWN_SECRETS {
        assert!(
            !public_json.contains(secret),
            "{secret} must be private-only"
        );
    }
}

#[test]
fn request_debug_and_redacted_json_never_contain_secrets() {
    let request = super::fixtures::transfer_request();
    let debug = format!("{request:?}");
    let redacted = request.redacted().to_string();
    for secret in KNOWN_SECRETS {
        assert!(!debug.contains(secret), "debug leaked {secret}: {debug}");
        assert!(
            !redacted.contains(secret),
            "redacted json leaked {secret}: {redacted}"
        );
    }
    // Names are fine; values are not.
    assert!(debug.contains("sender_sk"));
}

#[test]
fn prover_errors_never_contain_secrets() {
    // Force the validation error path and confirm no secret rides along.
    let mut request = super::fixtures::transfer_request();
    request.witness = crucible_interfaces::PrivateWitnessBag::new();
    let err = MockProver::new().generate(&request).unwrap_err();
    let text = err.to_string();
    for secret in KNOWN_SECRETS {
        assert!(!text.contains(secret), "error leaked {secret}: {text}");
    }
}

#[test]
fn proof_bytes_and_response_json_never_contain_secret_values() {
    let request = super::fixtures::transfer_request();
    let response = MockProver::new().generate(&request).unwrap();

    // The mock envelope embeds public context only; no private value may
    // appear as a plaintext substring of the proof payload.
    let payload = String::from_utf8_lossy(&response.proof.bytes);
    for secret in KNOWN_SECRETS {
        assert!(!payload.contains(secret), "proof payload leaked {secret}");
    }

    // The serialized response is all-public and must stay that way.
    let json = serde_json::to_string(&response).unwrap();
    for secret in KNOWN_SECRETS {
        assert!(!json.contains(secret), "response json leaked {secret}");
    }
}

#[test]
fn service_transcript_never_contains_secrets() {
    use crucible_prover_core::{TranscriptEntry, TranscriptWriter};
    use std::path::Path;

    let request = super::fixtures::transfer_request();
    let response = MockProver::new().generate(&request).unwrap();
    let outcome = super::stack()
        .verifier
        .verify(&crucible_interfaces::VerificationRequest::from_response(
            &response,
        ))
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.jsonl");
    let mut writer = TranscriptWriter::append(Path::new(&path)).unwrap();
    let entry = TranscriptEntry::from_request_response(&request, &response, &outcome);
    writer.write(&entry).unwrap();
    writer.flush().unwrap();
    drop(writer);

    let content = std::fs::read_to_string(&path).unwrap();
    for secret in KNOWN_SECRETS {
        assert!(!content.contains(secret), "transcript leaked {secret}");
    }
    // The transcript still records useful provenance.
    assert!(content.contains("transfer"));
    assert!(content.contains("verification_key_id"));
}
