//! Deterministic artifact identity.
//!
//! Every compiled circuit artifact must be findable and addressable without
//! guesswork: given a circuit and a version, tooling needs to know *where*
//! the artifact lives and *what* its manifest is called. This module encodes
//! that convention once so the CLI, the Noir adapter, and test-vector tooling
//! agree.

use crucible_interfaces::{CircuitId, Version};

/// Root directory under which compiled circuit artifacts are stored.
pub const ARTIFACT_ROOT: &str = "artifacts/circuits";

/// Manifest filename inside each artifact directory.
pub const MANIFEST_NAME: &str = "manifest.json";

/// The canonical relative path of an artifact directory for `circuit` at
/// `version`, e.g. `artifacts/circuits/transfer/0.1.0/`.
pub fn artifact_dir(circuit: &CircuitId, version: &Version) -> String {
    format!("{ARTIFACT_ROOT}/{circuit}/{version}")
}

/// The canonical path of an artifact's manifest JSON.
pub fn manifest_path(circuit: &CircuitId, version: &Version) -> String {
    format!("{}/{MANIFEST_NAME}", artifact_dir(circuit, version))
}

/// The canonical name of the ACIR bytecode file inside an artifact dir.
pub fn acir_filename() -> &'static str {
    "acir.bin"
}

/// The canonical name of the verification key file inside an artifact dir.
pub fn verification_key_filename() -> &'static str {
    "vk.bin"
}

/// A deterministic, human-readable verification key id for a circuit.
///
/// Real verification keys get their id from a key-management decision (see
/// the `ultrahonk` adapter); this convention gives tooling and fixtures a
/// stable default of the form `vk-<circuit>-<version>`.
pub fn default_verification_key_id(circuit: &CircuitId, version: &Version) -> String {
    format!("vk-{circuit}-{version}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_paths_are_deterministic() {
        let circuit = CircuitId::new("transfer").unwrap();
        let version = Version::new(1, 2, 3);
        assert_eq!(
            artifact_dir(&circuit, &version),
            "artifacts/circuits/transfer/1.2.3"
        );
        assert_eq!(
            manifest_path(&circuit, &version),
            "artifacts/circuits/transfer/1.2.3/manifest.json"
        );
        assert_eq!(
            default_verification_key_id(&circuit, &version),
            "vk-transfer-1.2.3"
        );
    }
}
