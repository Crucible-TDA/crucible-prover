//! Verification key identity for UltraHonk.
//!
//! An UltraHonk verification key is derived from the circuit bytecode, so its
//! identity is a digest of the compiled artifact, not a human label. This
//! module defines the policy for forming verification key ids from artifacts
//! and for parsing them back into `(circuit, version)`.

use crucible_interfaces::{ArtifactChecksum, CircuitId, Version};

use crate::errors::UltraHonkError;

/// Scheme prefix for UltraHonk verification key ids.
pub const VK_SCHEME: &str = "uhk";

/// Policy for forming and parsing UltraHonk verification key ids.
///
/// Ids look like `uhk/<circuit>/<version>/<artifact-hash>` — the artifact
/// hash (a SHA-256 of the compiled artifact, see `crucible-artifacts`) is the
/// actual key discriminator; circuit and version make the id human-debuggable
/// and enable fast mismatch detection before any cryptography runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationKeyIdPolicy;

impl VerificationKeyIdPolicy {
    /// Forms a verification key id for a circuit artifact.
    pub fn id_for(
        circuit: &CircuitId,
        version: &Version,
        artifact_hash: &ArtifactChecksum,
    ) -> String {
        format!("{VK_SCHEME}/{circuit}/{version}/{}", artifact_hash.as_hex())
    }

    /// Parses a verification key id back into its components.
    pub fn parse(id: &str) -> Result<(CircuitId, Version, ArtifactChecksum), UltraHonkError> {
        let parts: Vec<&str> = id.split('/').collect();
        if parts.len() != 4 || parts[0] != VK_SCHEME {
            return Err(UltraHonkError::BadVerificationKeyId { id: id.to_owned() });
        }
        let circuit = CircuitId::new(parts[1])
            .map_err(|_| UltraHonkError::BadVerificationKeyId { id: id.to_owned() })?;
        let version: Version = parts[2]
            .parse()
            .map_err(|_| UltraHonkError::BadVerificationKeyId { id: id.to_owned() })?;
        let hash = ArtifactChecksum::from_hex(parts[3])
            .map_err(|_| UltraHonkError::BadVerificationKeyId { id: id.to_owned() })?;
        Ok((circuit, version, hash))
    }

    /// Whether the id was produced under this scheme.
    pub fn is_valid(id: &str) -> bool {
        Self::parse(id).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        let circuit = CircuitId::for_operation(crucible_interfaces::Operation::Transfer);
        let version = Version::new(0, 1, 0);
        let hash = ArtifactChecksum::from_hex(&"ab".repeat(32)).unwrap();
        let id = VerificationKeyIdPolicy::id_for(&circuit, &version, &hash);
        let (parsed_circuit, parsed_version, parsed_hash) =
            VerificationKeyIdPolicy::parse(&id).unwrap();
        assert_eq!(parsed_circuit, circuit);
        assert_eq!(parsed_version, version);
        assert_eq!(parsed_hash, hash);
        assert!(VerificationKeyIdPolicy::is_valid(&id));
    }

    #[test]
    fn rejects_foreign_ids() {
        for bad in [
            "",
            "vk-transfer-0.1.0",              // other scheme
            "uhk/transfer/not-a-version/abc", // bad version
            "uhk/transfer/0.1.0/not-hex",     // bad hash
            "uhk/transfer",                   // too short
            "other/transfer/0.1.0/abc",       // wrong scheme
        ] {
            assert!(
                !VerificationKeyIdPolicy::is_valid(bad),
                "id should be invalid: {bad:?}"
            );
        }
    }
}
