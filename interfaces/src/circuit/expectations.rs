//! Per-operation circuit expectations.
//!
//! The protocol circuits (see `circuits/<op>/src/main.nr`) each expose a
//! small, fixed interface: private parameters, public parameters, and return
//! values, in declaration order. This module is the **single source of
//! truth** for those shapes on the Rust side, read by both the mock policy
//! (`crucible-prover-core`) and the real backend (`crucible-ultrahonk`), so
//! a circuit change that renames or reorders a parameter cannot silently
//! desynchronize the two.
//!
//! The names mirror the circuit sources exactly, and the ordering matters:
//! Barretenberg reports a proof's public inputs as *public parameters then
//! return values*, in declaration order, so naming those words requires this
//! exact list.

use super::Operation;

/// The expected interface of one protocol circuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircuitExpectations {
    /// Private parameter names, in declaration order.
    pub private: &'static [&'static str],
    /// Public parameter names, in declaration order.
    pub public_params: &'static [&'static str],
    /// Return value names, in declaration order.
    pub returns: &'static [&'static str],
}

impl CircuitExpectations {
    /// Every public name the proof commits to: parameters then returns, in
    /// the order the backend reports them.
    pub fn public_names(&self) -> impl Iterator<Item = &'static &'static str> {
        self.public_params.iter().chain(self.returns.iter())
    }

    /// Number of public input words the backend reports for this circuit
    /// (one per public parameter and return value).
    pub fn public_word_count(&self) -> usize {
        self.public_params.len() + self.returns.len()
    }
}

/// The expected interface of each protocol operation.
///
/// `Operation` is exactly the five protocol circuits, so this is total;
/// custom circuits are identified by [`super::CircuitId`] and have no
/// operation-level expectations.
pub fn expectations(operation: Operation) -> CircuitExpectations {
    match operation {
        Operation::Register => CircuitExpectations {
            private: &["account_sk"],
            public_params: &["account_address"],
            returns: &[],
        },
        Operation::Deposit => CircuitExpectations {
            private: &[
                "account_sk",
                "old_amount",
                "old_blinding",
                "amount",
                "blinding",
            ],
            public_params: &["token_address", "account_address", "old_commitment"],
            returns: &["new_commitment", "nullifier"],
        },
        Operation::Merge => CircuitExpectations {
            private: &[
                "account_sk",
                "amount_a",
                "blinding_a",
                "amount_b",
                "blinding_b",
                "blinding",
            ],
            public_params: &[
                "token_address",
                "account_address",
                "commitment_a",
                "commitment_b",
                "root_hi",
                "root_lo",
            ],
            returns: &["merged_commitment", "nullifier_a", "nullifier_b"],
        },
        Operation::Transfer => CircuitExpectations {
            private: &[
                "sender_sk",
                "amount",
                "old_amount",
                "old_blinding",
                "recipient_blinding",
                "change_blinding",
            ],
            public_params: &[
                "token_address",
                "sender_address",
                "recipient_address",
                "old_sender_commitment",
                "root_hi",
                "root_lo",
            ],
            returns: &["recipient_commitment", "change_commitment", "nullifier"],
        },
        Operation::Withdraw => CircuitExpectations {
            private: &[
                "account_sk",
                "amount",
                "old_amount",
                "old_blinding",
                "change_blinding",
            ],
            public_params: &[
                "token_address",
                "account_address",
                "commitment",
                "root_hi",
                "root_lo",
            ],
            returns: &["change_commitment", "nullifier"],
        },
    }
}

/// The private parameter names required by a protocol circuit.
pub fn private_names(operation: Operation) -> &'static [&'static str] {
    expectations(operation).private
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_protocol_operation_has_pinned_expectations() {
        for op in [
            Operation::Register,
            Operation::Deposit,
            Operation::Merge,
            Operation::Transfer,
            Operation::Withdraw,
        ] {
            let spec = expectations(op);
            assert!(!spec.public_params.is_empty(), "{op} needs public params");
            assert!(!spec.private.is_empty(), "{op} needs private params");
            // No name may appear twice across the whole surface.
            let mut seen: Vec<&str> = Vec::new();
            for name in spec
                .private
                .iter()
                .chain(spec.public_params.iter())
                .chain(spec.returns.iter())
            {
                assert!(!seen.contains(name), "{op}: duplicate name `{name}`");
                seen.push(name);
            }
        }
    }

    #[test]
    fn public_words_count_matches_known_shapes() {
        // register: 1 public param, no returns.
        assert_eq!(expectations(Operation::Register).public_word_count(), 1);
        // deposit: 3 params + 2 returns (state-unbound: token-bound nullifier only).
        assert_eq!(expectations(Operation::Deposit).public_word_count(), 5);
        // merge/transfer/withdraw are state-bound: their public surfaces add
        // the two state-root halves, so 6/6/5 params + 3/3/2 returns.
        assert_eq!(expectations(Operation::Merge).public_word_count(), 9);
        assert_eq!(expectations(Operation::Transfer).public_word_count(), 9);
        assert_eq!(expectations(Operation::Withdraw).public_word_count(), 7);
    }

    #[test]
    fn public_names_report_parameters_then_returns() {
        let names: Vec<&str> = expectations(Operation::Transfer).public_names().copied().collect();
        assert_eq!(
            names,
            vec![
                "token_address",
                "sender_address",
                "recipient_address",
                "old_sender_commitment",
                "root_hi",
                "root_lo",
                "recipient_commitment",
                "change_commitment",
                "nullifier",
            ]
        );
    }

    #[test]
    fn state_bound_operations_carry_root_params() {
        // The circuits' state-bound ops fold the ledger root halves into
        // their public surfaces; deposit/register remain token-bound only.
        assert!(expectations(Operation::Merge)
            .public_params
            .contains(&"root_hi"));
        assert!(expectations(Operation::Transfer)
            .public_params
            .contains(&"root_hi"));
        assert!(expectations(Operation::Withdraw)
            .public_params
            .contains(&"root_hi"));
        assert!(!expectations(Operation::Deposit)
            .public_params
            .contains(&"root_hi"));
        assert!(!expectations(Operation::Register)
            .public_params
            .contains(&"root_hi"));
    }
}
