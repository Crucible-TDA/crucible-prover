//! `crucible-prover prove` — build a witness from a test vector and
//! generate a proof envelope through [`ProverService`].
//!
//! The command is a thin facade over the same seams the integration suites
//! use: the request is assembled by the catalog loader, proving runs through
//! the service, and the result is written as a versioned [`ProofEnvelope`].
//! Witness material never touches the CLI's own code beyond the request it
//! hands to the service.

use std::path::{Path, PathBuf};

use clap::Args;
use crucible_interfaces::BackendId;
use crucible_prover_core::ProverService;
use crucible_vectors::TestVector;

use crate::paths;

#[derive(Debug, Args)]
pub struct ProveArgs {
    /// Operation circuit to prove (must match the vector's operation).
    pub op: String,
    /// Test-vector JSON file describing the witness (see
    /// `schemas/test-vector.schema.json`).
    #[arg(long, value_name = "PATH")]
    pub vector: PathBuf,
    /// Backend to prove with: `mock` (TEST ONLY, fast) or `ultrahonk`
    /// (real proofs; requires nargo + bb on PATH and compiled bytecode).
    #[arg(long, default_value = "mock", value_name = "BACKEND")]
    pub backend: String,
    /// Where to write the proof envelope JSON (defaults to
    /// `<vector-id>.<backend>.proof.json` in the current directory).
    #[arg(long, value_name = "PATH")]
    pub out: Option<PathBuf>,
    /// Verification-key store directory (ultrahonk backend; defaults to
    /// `<repo>/artifacts/verification-keys`).
    #[arg(long, value_name = "DIR")]
    pub vk_store: Option<PathBuf>,
}

pub fn run(args: ProveArgs, circuits: &Path) -> Result<(), String> {
    let vector = TestVector::load(&args.vector)
        .map_err(|e| format!("cannot load vector `{}`: {e}", args.vector.display()))?;
    if vector.operation.as_str() != args.op {
        return Err(format!(
            "operation `{}` does not match vector `{}` (operation {})",
            args.op,
            vector.id,
            vector.operation
        ));
    }
    if !vector.expect_verification {
        return Err(format!(
            "vector `{}` is a rejecting vector (expect_verification=false); \
             proving is only meaningful for witnesses expected to verify",
            vector.id
        ));
    }

    let request = vector.to_request_for(&args.backend);
    let mut service = ProverService::new(concat!("crucible-prover/", env!("CARGO_PKG_VERSION")));

    match args.backend.as_str() {
        BackendId::MOCK => {
            service.register_provider(Box::new(crucible_mock::MockProver::new()));
            service.with_local_verifier(Box::new(crucible_mock::MockVerifier::new()));
        }
        BackendId::ULTRAHONK => {
            if !crucible_noir::NoirToolchain::is_available()
                || !crucible_ultrahonk::BbToolchain::is_available()
            {
                return Err(
                    "ultrahonk proving requires nargo and bb on PATH (see \
                     scripts/check-bb.sh); use --backend mock for fast structural proving"
                        .to_owned(),
                );
            }
            let store = crucible_ultrahonk::VkStore::new(vk_store_or_default(&args.vk_store)?);
            let config = crucible_ultrahonk::UltraHonkConfig::new(circuits.to_path_buf(), store);
            let provider = crucible_ultrahonk::UltraHonkProvider::new(config);
            let verifier =
                crucible_ultrahonk::UltraHonkVerifier::new(provider.vk_store().clone());
            service.register_provider(Box::new(provider));
            service.with_local_verifier(Box::new(verifier));
        }
        other => {
            return Err(format!(
                "unknown backend `{other}` (expected `{}` or `{}`)",
                BackendId::MOCK,
                BackendId::ULTRAHONK
            ));
        }
    }

    let envelope = service
        .prove_enveloped_and_verify(&request)
        .map_err(|e| format!("proving failed: {e}"))?;

    let out = args.out.unwrap_or_else(|| {
        PathBuf::from(format!("{}.{}.proof.json", vector.id, args.backend))
    });
    let json = envelope
        .to_json()
        .map_err(|e| format!("serializing the proof envelope failed: {e}"))?;
    std::fs::write(&out, json).map_err(|e| format!("cannot write `{}`: {e}", out.display()))?;

    println!("request     {}", envelope.metadata.request_id);
    println!("circuit     {} v{}", envelope.circuit, envelope.circuit_version);
    println!("backend     {}", envelope.backend);
    println!("vk id       {}", envelope.verification_key_id);
    println!(
        "public      {} word(s)",
        envelope.public_outputs.len()
    );
    println!(
        "state       {}",
        envelope
            .state_reference
            .as_ref()
            .map(|s| format!("root {}", s.root))
            .unwrap_or_else(|| "unbound".to_owned())
    );
    println!("verified    yes (local round-trip)");
    println!("envelope    {}", out.display());
    Ok(())
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