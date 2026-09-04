//! `crucible-prover verify` — verify a proof envelope.
//!
//! The envelope is self-describing (backend, verification-key id, public
//! outputs, state reference), so the command can dispatch to the matching
//! verifier without user hints: mock envelopes go to `MockVerifier`,
//! ultrahonk envelopes to `UltraHonkVerifier` resolving the key from the
//! store. A rejected proof exits non-zero with the failure reason — the
//! caller-visible equivalent of the integration suites' rejection paths.

use std::path::PathBuf;

use clap::Args;
use crucible_interfaces::{
    VerificationRequest, Verifier,
};

use crate::paths;

#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// Proof envelope JSON file produced by `crucible-prover prove`.
    pub envelope: PathBuf,
    /// Verification-key store directory (ultrahonk backend; defaults to
    /// `<repo>/artifacts/verification-keys`).
    #[arg(long, value_name = "DIR")]
    pub vk_store: Option<PathBuf>,
}

pub fn run(args: VerifyArgs) -> Result<(), String> {
    let text = std::fs::read_to_string(&args.envelope)
        .map_err(|e| format!("cannot read `{}`: {e}", args.envelope.display()))?;
    let envelope = crucible_proof_types::ProofEnvelope::from_json(&text)
        .map_err(|e| format!("`{}` is not a valid proof envelope: {e}", args.envelope.display()))?;

    let request = VerificationRequest::new(
        envelope.metadata.request_id.clone(),
        envelope.proof.clone(),
        envelope.circuit.clone(),
        envelope.circuit_version,
        envelope.backend.clone(),
        envelope.verification_key_id.clone(),
        envelope.artifact_checksum.clone(),
        envelope.public_outputs.clone(),
        envelope.state_reference.clone(),
    );

    let outcome = match envelope.backend.as_str() {
        "mock" => crucible_mock::MockVerifier::new().verify(&request),
        "ultrahonk" => {
            let store = crucible_ultrahonk::VkStore::new(vk_store_or_default(&args.vk_store)?);
            crucible_ultrahonk::UltraHonkVerifier::new(store).verify(&request)
        }
        other => {
            return Err(format!(
                "envelope claims unknown backend `{other}`; nothing is registered to verify it"
            ));
        }
    }
    .map_err(|e| format!("verification could not run: {e}"))?;

    if outcome.verified {
        println!(
            "verified: proof {} for circuit {} v{} ({})",
            envelope.metadata.request_id, envelope.circuit, envelope.circuit_version, envelope.backend
        );
        Ok(())
    } else {
        let reason = outcome
            .failure
            .map(|f| format!("{f}"))
            .unwrap_or_else(|| "unknown".to_owned());
        Err(format!(
            "proof {} rejected for circuit {} v{} ({}): {reason}",
            envelope.metadata.request_id, envelope.circuit, envelope.circuit_version, envelope.backend
        ))
    }
}

/// Resolves the verification-key store directory, creating it on demand.
fn vk_store_or_default(explicit: &Option<PathBuf>) -> Result<PathBuf, String> {
    let dir = explicit
        .clone()
        .unwrap_or_else(paths::default_vk_store);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("cannot create verification-key store `{}`: {e}", dir.display()))?;
    Ok(dir)
}