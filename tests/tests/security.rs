//! Security suite: adversarial scenarios against the composed stack.
//!
//! These tests generate real (mock) proofs and then attack them: tampering,
//! replay, stale-state submission, wrong context, witness leakage, and
//! artifact tampering. They map one-to-one onto the threat model in
//! `docs/threat-model.md`.

use crucible_mock::fixtures;

#[path = "security/artifact_tampering.rs"]
mod artifact_tampering;
#[path = "security/key_mismatch.rs"]
mod key_mismatch;
#[path = "security/proof_malleability.rs"]
mod proof_malleability;
#[path = "security/replay.rs"]
mod replay;
#[path = "security/stale_state.rs"]
mod stale_state;
#[path = "security/witness_leakage.rs"]
mod witness_leakage;
#[path = "security/wrong_context.rs"]
mod wrong_context;

use crucible_tests::MockStack;

/// A stack used by every security test: one prover + one matching verifier.
pub(crate) fn stack() -> MockStack {
    MockStack::new()
}

/// A valid transfer response, freshly generated.
pub(crate) fn transfer_response() -> crucible_interfaces::ProofResponse {
    stack().prove(&fixtures::transfer_request())
}

#[test]
fn security_suite_builds_valid_baseline() {
    let response = transfer_response();
    assert!(stack().verify(&response).verified);
}
