//! Witness orchestration.
//!
//! The [`crucible-witness`] crate owns the *mechanics* of witness handling
//! (assembly, encoding, decoding). This module owns the *policy*: which
//! private and public names a standard protocol circuit requires. It is the
//! single place a request's witness is checked against circuit expectations
//! before dispatch, keeping that knowledge out of the provider backends.

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

/// Public parameter names required by each protocol circuit.
///
/// These mirror the circuit `main`'s `pub` parameters: the public context a
/// proof must bind to. Since batch 6, the state-bound operations
/// (merge/transfer/withdraw) require `root_hi`/`root_lo` — the two halves
/// of the ledger state root the operation runs against — so a request for
/// those operations that omits the root halves could never produce a proof
/// that verifies under the submitted state. The list lives in the same
/// expectations spec as the private names.
pub fn required_public_names(operation: Operation) -> &'static [&'static str] {
    crucible_interfaces::circuit::expectations(operation).public_params
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

/// Checks that `request` carries every public parameter its operation's
/// circuit declares. Returns the list of missing names (empty when valid).
pub fn missing_public_names(request: &ProofRequest) -> Vec<&'static str> {
    required_public_names(request.operation)
        .iter()
        .copied()
        .filter(|name| request.public_inputs.get(name).is_none())
        .collect()
}

/// Whether the request's witness satisfies the circuit's requirements.
pub fn has_required_witness(request: &ProofRequest) -> bool {
    missing_private_names(request).is_empty()
}

/// Whether the request carries every public parameter the circuit declares.
pub fn has_required_public_inputs(request: &ProofRequest) -> bool {
    missing_public_names(request).is_empty()
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

    #[test]
    fn missing_public_params_are_reported() {
        let mut request = fixtures::transfer_request();
        request.public_inputs = PublicInputBag::new();
        let missing = missing_public_names(&request);
        assert_eq!(
            missing,
            vec![
                "token_address",
                "sender_address",
                "recipient_address",
                "old_sender_commitment",
                "root_hi",
                "root_lo",
            ]
        );
        assert!(!has_required_public_inputs(&request));
    }

    #[test]
    fn state_bound_fixtures_carry_root_halves() {
        for request in fixtures::all_valid_requests() {
            assert!(
                has_required_public_inputs(&request),
                "{} must carry every declared public param: {:?}",
                request.operation,
                missing_public_names(&request)
            );
        }
    }
}
