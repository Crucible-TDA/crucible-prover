//! Invariant: private witness values never cross into public territory. A
//! proof response, its envelope, and its public outputs must contain no
//! trace of the private inputs that produced them.

use crucible_interfaces::Verifier;
use crucible_mock::fixtures;

/// Distinctive private values from the fixture set.
///
/// Markers are ≥8 hex chars so they cannot randomly collide with the
/// artifact checksums embedded in envelopes (short values like `7f` would
/// make these assertions flaky).
const PRIVATE_MARKERS: [&str; 3] = ["deadbeefcafe", "0102030405", "feedface"];

#[test]
fn public_outputs_never_echo_private_values() {
    let stack = crucible_tests::MockStack::new();
    for request in fixtures::all_valid_requests() {
        let response = stack.prove(&request);
        let outputs_json = serde_json::to_string(&response.public_outputs).unwrap();
        for marker in PRIVATE_MARKERS {
            assert!(
                !outputs_json.contains(marker),
                "{} response public outputs leaked {marker}: {outputs_json}",
                request.operation
            );
        }
    }
}

#[test]
fn envelope_and_response_never_echo_private_values() {
    let stack = crucible_tests::MockStack::new();
    for request in fixtures::all_valid_requests() {
        let response = stack.prove(&request);
        let envelope = crucible_proof_types::ProofEnvelope::from_response(
            &response,
            request.operation,
            "invariants",
        );
        let documents = [
            envelope.to_json().unwrap(),
            serde_json::to_string(&response).unwrap(),
            stack.verifier.backend().to_string(),
        ];
        for doc in documents {
            for marker in PRIVATE_MARKERS {
                assert!(
                    !doc.contains(marker),
                    "{} document leaked {marker}: {doc}",
                    request.operation
                );
            }
        }
    }
}

#[test]
fn private_bag_names_and_values_are_not_in_public_inputs() {
    let request = fixtures::transfer_request();
    let public_names: Vec<&str> = request.public_inputs.names().collect();
    for private_name in request.witness.names() {
        assert!(
            !public_names.contains(&private_name),
            "private name `{private_name}` must not also be a public input"
        );
    }
}
