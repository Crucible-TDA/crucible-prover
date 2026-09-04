//! The canonical circuit catalog.
//!
//! These helpers centralize "which circuits exist and what they are called"
//! so that tooling (the CLI, artifact layout, test vectors) never hard-codes
//! operation strings. The catalog is deliberately small: the exact circuit
//! interfaces are defined by the Confidential Token architecture, and this
//! module only names the five protocol operations plus the convention that a
//! circuit id equals its operation name.

use crucible_interfaces::{CircuitId, Operation, Version};

/// The five protocol operations, in canonical order.
pub const PROTOCOL_OPERATIONS: [Operation; 5] = [
    Operation::Register,
    Operation::Deposit,
    Operation::Merge,
    Operation::Transfer,
    Operation::Withdraw,
];

/// The current circuit version for the standard circuits.
pub const CURRENT_CIRCUIT_VERSION: Version = Version::new(0, 1, 0);

/// Returns the canonical circuit id for every protocol operation.
pub fn protocol_circuit_ids() -> Vec<CircuitId> {
    PROTOCOL_OPERATIONS
        .iter()
        .map(|op| CircuitId::for_operation(*op))
        .collect()
}

/// Whether `circuit` is one of the five standard protocol circuits.
pub fn is_protocol_circuit(circuit: &CircuitId) -> bool {
    PROTOCOL_OPERATIONS
        .iter()
        .any(|op| CircuitId::for_operation(*op) == *circuit)
}

/// Whether `version` is the currently supported version of the standard
/// circuits. Custom/experimental circuits may use other versions; this is a
/// *catalog* convention, not a global rule.
pub fn is_current_version(version: &Version) -> bool {
    *version == CURRENT_CIRCUIT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_matches_operation_names() {
        for op in PROTOCOL_OPERATIONS {
            let id = CircuitId::for_operation(op);
            assert_eq!(id.as_str(), op.as_str());
            assert!(is_protocol_circuit(&id));
        }
        assert!(!is_protocol_circuit(&CircuitId::new("custom").unwrap()));
        assert!(is_current_version(&CURRENT_CIRCUIT_VERSION));
        assert!(!is_current_version(&Version::new(1, 0, 0)));
        assert_eq!(protocol_circuit_ids().len(), 5);
    }
}
