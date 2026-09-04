use serde::{Deserialize, Serialize};

use super::{CircuitId, Operation, Version};

/// Immutable description of a circuit in the Crucible catalog.
///
/// Metadata answers the first question of artifact management: *which*
/// circuit is this, at *which* version, implementing *which* operation? It is
/// attached to compiled artifacts, manifests, proofs, and verification keys
/// so every artifact is traceable to a circuit identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitMetadata {
    /// The circuit's identity in the catalog (e.g. `transfer`).
    pub id: CircuitId,
    /// The protocol operation this circuit proves.
    pub operation: Operation,
    /// The circuit's semantic version.
    pub version: Version,
}

impl CircuitMetadata {
    /// Creates circuit metadata.
    pub fn new(id: CircuitId, operation: Operation, version: Version) -> CircuitMetadata {
        CircuitMetadata {
            id,
            operation,
            version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_round_trips_through_json() {
        let meta = CircuitMetadata::new(
            CircuitId::for_operation(Operation::Transfer),
            Operation::Transfer,
            Version::v0_1(),
        );
        let json = serde_json::to_string_pretty(&meta).unwrap();
        let back: CircuitMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back, meta);
        assert!(json.contains("\"transfer\""));
        assert!(json.contains("\"0.1.0\""));
    }
}
