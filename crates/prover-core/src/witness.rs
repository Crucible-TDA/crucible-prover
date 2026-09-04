//! Witness orchestration.
//!
//! The [`crucible-witness`] crate owns the *mechanics* of witness handling
//! (assembly, encoding, decoding). This module owns the *policy*: which
//! private names a standard protocol circuit requires. It is the single
//! place a request's witness is checked against circuit expectations before
//! dispatch, keeping that knowledge out of the provider backends.

use crucible_interfaces::{Operation, ProofRequest, PublicInputBag};

/// Private witness names required by each protocol circuit.
///
/// These are the **private parameters of the circuit's `main`** (see
/// `circuits/<op>/src/main.nr`): the values a prover must supply beyond the
/// public context. Registration proves knowledge of an account secret;
/// deposit/merge/transfer/withdraw prove knowledge of the secret(s) and the
/// amounts/blindings guarding the commitments being moved. The list itself
/// lives in [`crucible_interfaces::circuit::expectations`] — the single
/// source of truth shared with the real backend — and this module is the
/// preflight policy that keeps the names out of the provider backends:
/// a request missing any of these names could never produce a satisfying
/// witness, regardless of backend.
pub fn required_private_names(operation: Operation) -> &'static [&'static str] {
    crucible_interfaces::circuit::private_names(operation)
}

/// Checks that `request` carries every private witness name its operation
/// requires. Returns the list of missing names (empty when valid).
pub fn missing_private_names(request: &ProofRequest) -> Vec<&'static str> {
    required_private_names(request.operation)
        .iter()
        .copied()
        .filter(|name| request.witness.get(name).is_none())
        .collect()
}

/// Whether the request's witness satisfies the circuit's requirements.
pub fn has_required_witness(request: &ProofRequest) -> bool {
    missing_private_names(request).is_empty()
}

/// Returns the public inputs of `request` (they double as the expected
/// outputs in the standard flow).
pub fn public_inputs(request: &ProofRequest) -> &PublicInputBag {
    &request.public_inputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::PrivateWitnessBag;
    use crucible_mock::fixtures;

    #[test]
    fn standard_fixtures_satisfy_requirements() {
        for request in fixtures::all_valid_requests() {
            assert!(
                has_required_witness(&request),
                "{} must carry its required witness names: {:?}",
                request.operation,
                missing_private_names(&request)
            );
        }
    }

    #[test]
    fn missing_names_are_reported() {
        let mut request = fixtures::transfer_request();
        request.witness = PrivateWitnessBag::new();
        let missing = missing_private_names(&request);
        assert_eq!(
            missing,
            vec![
                "sender_sk",
                "amount",
                "old_amount",
                "old_blinding",
                "recipient_blinding",
                "change_blinding"
            ]
        );
        assert!(!has_required_witness(&request));
    }
}
