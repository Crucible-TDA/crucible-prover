//! Deterministic, valid request fixtures for every protocol operation.
//!
//! These exist so that unit, integration, and security tests across the
//! workspace (and `crucible-scenarios`) never re-derive "a valid request" by
//! hand. Every fixture is deterministic: the same function always returns the
//! same request, which keeps mock proofs reproducible.

use crucible_interfaces::{
    BackendId, CircuitId, FieldValue, Operation, PrivateWitnessBag, ProofRequest, PublicInputBag,
    RequestId, RootDigest, SecretValue, StateReference, Version,
};

/// A fixed state root shared by state-bound fixtures.
pub fn state_root_a() -> RootDigest {
    RootDigest::from_hex(&"ab".repeat(32)).expect("valid hex")
}

/// `root_hi` for [`state_root_a`] — the most significant 128 bits (`ab`*16).
pub fn state_root_a_hi() -> &'static str {
    "abababababababababababababababab"
}

/// `root_lo` for [`state_root_a`] — the least significant 128 bits (`ab`*16).
pub fn state_root_a_lo() -> &'static str {
    "abababababababababababababababab"
}

/// A second, distinct state root for stale-state tests.
pub fn state_root_b() -> RootDigest {
    RootDigest::from_hex(&"cd".repeat(32)).expect("valid hex")
}

fn state(tag: &str) -> StateReference {
    StateReference::with_label(state_root_a(), 1, tag)
}

fn base_request(operation: Operation, tag: &str) -> ProofRequest {
    ProofRequest::new(
        RequestId::new(format!("req-{tag}")),
        operation,
        CircuitId::for_operation(operation),
        Version::v0_1(),
        Version::v0_1(),
        BackendId::new(BackendId::MOCK).expect("valid backend"),
        PrivateWitnessBag::new(),
        PublicInputBag::new(),
        None,
    )
}

fn with_secret(mut request: ProofRequest, name: &str, hex: &str) -> ProofRequest {
    request
        .witness
        .insert(name, SecretValue::from_hex(hex).expect("valid hex"))
        .expect("no duplicate name");
    request
}

fn with_public(mut request: ProofRequest, name: &str, hex: &str) -> ProofRequest {
    request
        .public_inputs
        .insert(name, FieldValue::from_hex(hex).expect("valid hex"))
        .expect("no duplicate name");
    request
}

fn with_state(mut request: ProofRequest, state: StateReference) -> ProofRequest {
    request.state_reference = Some(state);
    request
}

/// A valid register request: an account is created with an opening and an
/// initial nullifier key. No prior state is required.
pub fn register_request() -> ProofRequest {
    with_secret(
        with_public(
            base_request(Operation::Register, "register-1"),
            "account_address",
            "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        ),
        "account_sk",
        "0x112233445566778899aabbccddeeff00",
    )
}

/// A valid deposit request: value enters the confidential domain for an
/// existing account, producing a new commitment. Bound to prior state.
///
/// Private witness names mirror the real circuit's `main` parameters
/// (`circuits/deposit/src/main.nr`); values are synthetic.
pub fn deposit_request() -> ProofRequest {
    with_state(
        with_secret(
            with_secret(
                with_secret(
                    with_secret(
                        with_secret(
                            with_public(
                                with_public(
                                    with_public(
                                        base_request(Operation::Deposit, "deposit-1"),
                                        "token_address",
                                        "c0ffee",
                                    ),
                                    "account_address",
                                    "a1b2c3d4",
                                ),
                                "old_commitment",
                                "aa11",
                            ),
                            "account_sk",
                            "0x1122334455667788",
                        ),
                        "old_amount",
                        "0x3e8",
                    ),
                    "old_blinding",
                    "0x05",
                ),
                "amount",
                "0x1000",
            ),
            "blinding",
            "0xabcdef",
        ),
        state("deposit-1"),
    )
}

/// A valid merge request: multiple commitments consolidate into one.
/// Bound to prior state.
///
/// Private witness names mirror the real circuit's `main` parameters
/// (`circuits/merge/src/main.nr`); values are synthetic.
pub fn merge_request() -> ProofRequest {
    with_state(
        with_secret(
            with_secret(
                with_secret(
                    with_secret(
                        with_secret(
                            with_secret(
                                with_public(
                                    with_public(
                                        with_public(
                                            with_public(
                                                with_public(
                                                    with_public(
                                                        base_request(Operation::Merge, "merge-1"),
                                                        "token_address",
                                                        "c0ffee",
                                                    ),
                                                    "account_address",
                                                    "a1b2c3d4",
                                                ),
                                                "commitment_a",
                                                "1111",
                                            ),
                                            "commitment_b",
                                            "2222",
                                        ),
                                        "root_hi",
                                        state_root_a_hi(),
                                    ),
                                    "root_lo",
                                    state_root_a_lo(),
                                ),
                                "account_sk",
                                "0x1122334455667788",
                            ),
                            "amount_a",
                            "0x258",
                        ),
                        "blinding_a",
                        "0x07",
                    ),
                    "amount_b",
                    "0x190",
                ),
                "blinding_b",
                "0x0b",
            ),
            "blinding",
            "0x0d",
        ),
        state("merge-1"),
    )
}

/// A valid transfer request: the most security-critical fixture.
/// Sender and recipient are public; balances and amount are private.
/// Bound to prior state.
///
/// Private witness names mirror the real circuit's `main` parameters
/// (`circuits/transfer/src/main.nr`); values are synthetic.
pub fn transfer_request() -> ProofRequest {
    with_state(
        with_secret(
            with_secret(
                with_secret(
                    with_secret(
                        with_secret(
                            with_secret(
                                with_public(
                                    with_public(
                                        with_public(
                                            with_public(
                                                with_public(
                                                    with_public(
                                                        base_request(
                                                            Operation::Transfer,
                                                            "transfer-1",
                                                        ),
                                                        "token_address",
                                                        "c0ffee",
                                                    ),
                                                    "sender_address",
                                                    "aa11",
                                                ),
                                                "recipient_address",
                                                "bb22",
                                            ),
                                            "old_sender_commitment",
                                            "cc33",
                                        ),
                                        "root_hi",
                                        state_root_a_hi(),
                                    ),
                                    "root_lo",
                                    state_root_a_lo(),
                                ),
                                "sender_sk",
                                "0xdeadbeefcafe",
                            ),
                            "amount",
                            "0x7f",
                        ),
                        "old_amount",
                        "0x3e8",
                    ),
                    "old_blinding",
                    "0x05",
                ),
                "recipient_blinding",
                "0x11",
            ),
            "change_blinding",
            "0x0102030405",
        ),
        state("transfer-1"),
    )
}

/// A valid withdraw request: value leaves the confidential domain.
/// Bound to prior state.
///
/// Private witness names mirror the real circuit's `main` parameters
/// (`circuits/withdraw/src/main.nr`); values are synthetic.
pub fn withdraw_request() -> ProofRequest {
    with_state(
        with_secret(
            with_secret(
                with_secret(
                    with_secret(
                        with_secret(
                            with_public(
                                with_public(
                                    with_public(
                                        with_public(
                                            with_public(
                                                base_request(Operation::Withdraw, "withdraw-1"),
                                                "token_address",
                                                "c0ffee",
                                            ),
                                            "account_address",
                                            "dd44",
                                        ),
                                        "commitment",
                                        "ee55",
                                    ),
                                    "root_hi",
                                    state_root_a_hi(),
                                ),
                                "root_lo",
                                state_root_a_lo(),
                            ),
                            "account_sk",
                            "0x1020304050607080",
                        ),
                        "amount",
                        "0x3e8",
                    ),
                    "old_amount",
                    "0x3e8",
                ),
                "old_blinding",
                "0x05",
            ),
            "change_blinding",
            "0xfeedface",
        ),
        state("withdraw-1"),
    )
}

/// Returns one valid request per operation, covering the whole catalog.
pub fn all_valid_requests() -> Vec<ProofRequest> {
    vec![
        register_request(),
        deposit_request(),
        merge_request(),
        transfer_request(),
        withdraw_request(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::RootDigest;

    #[test]
    fn every_fixture_passes_structural_validation() {
        for request in all_valid_requests() {
            assert!(
                request.validate().is_ok(),
                "{} fixture must be structurally valid",
                request.operation
            );
        }
    }

    #[test]
    fn fixtures_are_deterministic() {
        // ProofRequest is intentionally not PartialEq (it carries secrets);
        // the redacted JSON view is deterministic and safe to compare.
        assert_eq!(transfer_request().redacted(), transfer_request().redacted());
        assert_eq!(register_request().redacted(), register_request().redacted());
    }

    #[test]
    fn distinct_operations_have_distinct_circuit_ids() {
        for op in [
            Operation::Register,
            Operation::Deposit,
            Operation::Merge,
            Operation::Transfer,
            Operation::Withdraw,
        ] {
            assert_eq!(CircuitId::for_operation(op).as_str(), op.as_str());
        }
    }

    #[test]
    fn state_roots_are_distinct() {
        assert_ne!(state_root_a(), state_root_b());
        assert_eq!(state_root_a().as_hex().len(), 64);
        assert_eq!(
            state_root_b(),
            RootDigest::from_hex(&"cd".repeat(32)).unwrap()
        );
    }

    #[test]
    fn state_bound_fixtures_carry_the_reference() {
        assert!(transfer_request().state_reference.is_some());
        assert!(merge_request().state_reference.is_some());
        assert!(withdraw_request().state_reference.is_some());
        assert!(deposit_request().state_reference.is_some());
    }
}
