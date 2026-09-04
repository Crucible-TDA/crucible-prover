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
/// These names follow the Confidential Token operation model: registration
/// proves knowledge of an account secret; deposit/merge/transfer/withdraw
/// prove knowledge of the secret(s) guarding the commitments being moved.
/// The exact per-operation name sets will converge with the real circuits
/// (see `circuits/`); today they are the contract fixtures exercise.
pub fn required_private_names(operation: Operation) -> &'static [&'static str] {
    match operation {
        Operation::Register => &["account_sk"],
        Operation::Deposit => &["amount", "blinding"],
        Operation::Merge => &["opening_a", "opening_b"],
        Operation::Transfer => &["sender_sk", "amount", "blinding"],
        Operation::Withdraw => &["account_sk", "amount", "blinding"],
    }
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
        assert_eq!(missing, vec!["sender_sk", "amount", "blinding"]);
        assert!(!has_required_witness(&request));
    }
}
