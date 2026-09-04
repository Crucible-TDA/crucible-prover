//! Vector runner: executes the `test-vectors/` catalog.
//!
//! Two tiers run against every catalog entry:
//!
//! 1. **Mock tier** (always): every vector must produce a structurally valid
//!    [`ProofRequest`], and `valid` vectors must round-trip through the mock
//!    stack (prove → verify). The mock is semantically blind, so it cannot
//!    judge *why* a rejecting vector fails — that is the circuit tier's job.
//! 2. **Circuit tier** (when `nargo` is on PATH): each vector's witness is
//!    written as a `Prover.toml` and executed against the real Noir circuit
//!    package. Valid vectors must solve and report exactly the fixture's
//!    expected outputs; rejecting vectors must fail to solve.
//!
//! A vector whose JSON contradicts either tier is a catalog bug, not a code
//! bug: the fixture's whole purpose is to pin cross-language expectations.

use crucible_interfaces::prover::Prover;
use crucible_interfaces::{RootDigest, StateReference, VerificationFailure};
use crucible_noir::NoirToolchain;
use crucible_tests::MockStack;
use crucible_tests::circuit_runner::{
    CircuitScratch, assert_outputs_match, circuits_root, execute_vector,
};
use crucible_tests::vectors::{TestVector, load_catalog};
use std::collections::HashMap;
use std::path::Path;

fn catalog() -> Vec<TestVector> {
    load_catalog(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors"))
        .expect("vector catalog must load")
}

/// Tier 1: every valid vector builds a valid request and round-trips through
/// the mock stack (prove → verify).
#[test]
fn mock_tier_valid_vectors_round_trip() {
    let stack = MockStack::new();
    for vector in catalog().iter().filter(|v| v.expect_verification) {
        let request = vector.to_request();
        request
            .validate()
            .expect("valid vector must form a valid request");
        let response = stack.prove(&request);
        assert_eq!(response.circuit.as_str(), vector.circuit.as_str());
        assert_eq!(response.circuit_version, vector.circuit_version);
        let outcome = stack.verify(&response);
        assert!(
            outcome.verified,
            "vector {} must verify: {outcome}",
            vector.id
        );
    }
}

/// Tier 1 (rejecting side): rejecting vectors are *well-formed* requests —
/// the backend must reject them at witness time, never because the request
/// itself is malformed. This test pins that distinction by proving every
/// rejecting vector can at least be *expressed* as a request through the mock
/// stack (which is semantically blind and therefore must not be used to judge
/// them).
#[test]
fn mock_tier_rejecting_vectors_are_wellformed() {
    let stack = MockStack::new();
    for vector in catalog().iter().filter(|v| !v.expect_verification) {
        let request = vector.to_request();
        request.validate().unwrap_or_else(|e| {
            panic!(
                "rejecting vector {} must still be well-formed: {e}",
                vector.id
            )
        });
        // Prove without asserting verification: the mock accepts any
        // well-formed request, so we only pin expressibility here.
        let _ = stack.service.prove(&request).unwrap_or_else(|e| {
            panic!(
                "rejecting vector {} must be provable by the mock: {e}",
                vector.id
            )
        });
    }
}

/// Tier 2 (circuit): valid vectors solve and report fixture outputs exactly.
#[test]
fn circuit_tier_valid_vectors_solve_with_expected_outputs() {
    if !NoirToolchain::is_available() {
        eprintln!("skipping: nargo not installed");
        return;
    }
    let toolchain = NoirToolchain::locate().expect("nargo available");
    let by_op = group_by_operation(&catalog());

    for (op, vectors) in by_op {
        let valid: Vec<_> = vectors.iter().filter(|v| v.expect_verification).collect();
        if valid.is_empty() {
            continue;
        }
        let scratch = CircuitScratch::prepare(&op).expect("scratch package prepared");
        for vector in valid {
            let verdict = execute_vector(&toolchain, &scratch, vector)
                .unwrap_or_else(|e| panic!("vector {} execution: {e}", vector.id));
            assert!(
                verdict.solved,
                "valid vector {} must solve the {} circuit (stdout: {})",
                vector.id,
                op,
                verdict.reported.values.join(", ")
            );
            assert_outputs_match(vector, &verdict.reported).unwrap_or_else(|e| panic!("{e}"));
        }
    }
}

/// Tier 2 (circuit): rejecting vectors must fail to solve.
#[test]
fn circuit_tier_rejecting_vectors_do_not_solve() {
    if !NoirToolchain::is_available() {
        eprintln!("skipping: nargo not installed");
        return;
    }
    let toolchain = NoirToolchain::locate().expect("nargo available");
    let by_op = group_by_operation(&catalog());

    for (op, vectors) in by_op {
        let rejecting: Vec<_> = vectors.iter().filter(|v| !v.expect_verification).collect();
        if rejecting.is_empty() {
            continue;
        }
        let scratch = CircuitScratch::prepare(&op).expect("scratch package prepared");
        for vector in rejecting {
            let verdict = execute_vector(&toolchain, &scratch, vector)
                .unwrap_or_else(|e| panic!("vector {} execution: {e}", vector.id));
            assert!(
                !verdict.solved,
                "rejecting vector {} must NOT solve the {} circuit",
                vector.id, op
            );
        }
    }
}

/// Every operation has at least one vector in each direction (structural
/// catalog sanity is covered in the loader tests; this asserts the *runner*
/// sees both).
#[test]
fn catalog_covers_every_operation_in_both_directions() {
    let by_op = group_by_operation(&catalog());
    for op in ["register", "deposit", "merge", "transfer", "withdraw"] {
        let vectors = by_op.get(op).expect("operation present in catalog");
        assert!(
            vectors.iter().any(|v| v.expect_verification),
            "{op}: missing valid vector"
        );
        assert!(
            vectors.iter().any(|v| !v.expect_verification),
            "{op}: missing rejecting vector"
        );
    }
}

/// Cross-operation replay protection: a valid transfer proof bound to state
/// root A must be rejected when the submission context moves to root B. This
/// is exercised at the proof layer via the mock (the same state-binding the
/// security suite tests) but driven from the vector catalog's real witness.
#[test]
fn replay_protection_binds_valid_vector_to_state() {
    let vectors = catalog();
    let transfer = vectors
        .iter()
        .find(|v| v.id == "transfer-valid-001")
        .expect("transfer-valid-001 exists");
    assert!(transfer.expect_verification);

    let stack = MockStack::new();
    // Prove under the vector's own state root.
    let request_a = transfer.to_request();
    let response_a = stack.prove(&request_a);

    // Re-submit the same proof against state root B: must be rejected.
    let root_b = StateReference::new(
        RootDigest::from_hex(&"cd".repeat(32)).expect("valid hex"),
        2,
    );
    let mut stale = response_a;
    stale.state_reference = Some(root_b);
    let outcome_b = stack.verify(&stale);
    assert!(
        outcome_b.rejected_with(VerificationFailure::StateReferenceMismatch),
        "proof cut for root A must be rejected under root B: {outcome_b}"
    );
}

/// Same-context determinism: proving the same valid vector twice yields the
/// same proof bytes (a property fixture execution relies on).
#[test]
fn deterministic_proofs_for_valid_vectors() {
    let vectors = catalog();
    let deposit = vectors
        .iter()
        .find(|v| v.id == "deposit-valid-001")
        .expect("deposit-valid-001 exists");
    let stack = MockStack::new();
    let request = deposit.to_request();
    let first = stack.prove(&request);
    let second = stack.prove(&request);
    assert_eq!(first.proof.bytes, second.proof.bytes);
}

fn group_by_operation(vectors: &[TestVector]) -> HashMap<String, Vec<TestVector>> {
    let mut map: HashMap<String, Vec<TestVector>> = HashMap::new();
    for v in vectors {
        map.entry(v.circuit.as_str().to_owned())
            .or_default()
            .push(v.clone());
    }
    map
}

// Silence unused-import warnings when nargo is absent is not a thing — these
// are used in the gated tests above.
#[allow(unused)]
fn _assert_circuits_present() {
    assert!(circuits_root().is_dir(), "circuits workspace must exist");
}
