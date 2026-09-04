//! UltraHonk proof format identity.

/// Canonical proof-format tag for UltraHonk proofs.
///
/// This matches the [`crucible_interfaces::ProofFormat::ULTRAHONK`]
/// convention so proofs self-describe as UltraHonk on the wire.
pub const PROOF_FORMAT_TAG: &str = "ultrahonk-v1";

/// A human label for documentation and manifests.
pub const PROOF_FORMAT: &str = "UltraHonk (Barretenberg)";

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::ProofFormat;

    #[test]
    fn tag_matches_the_interface_convention() {
        assert_eq!(PROOF_FORMAT_TAG, ProofFormat::ULTRAHONK);
        assert_eq!(
            ProofFormat::new(PROOF_FORMAT_TAG).unwrap().as_str(),
            PROOF_FORMAT_TAG
        );
    }
}
