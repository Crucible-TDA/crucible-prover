//! `crucible-prover vectors run` — judge the test-vector catalog through
//! the mock tier.
//!
//! Mirrors the integration suite's mock-tier semantics exactly:
//!
//! - vectors expected to verify must form a valid [`ProofRequest`],
//!   round-trip through the mock stack, and verify;
//! - rejecting vectors must still be *well-formed* requests (the backend
//!   must reject them at witness-solve time, never because the request is
//!   malformed), so they are proven but not expected to verify.
//!
//! The circuit tier (real nargo judging of witnesses and reported outputs)
//! lives in the test suite: `cargo test -p crucible-tests --test vectors`.

use std::path::PathBuf;

use clap::Subcommand;
use crucible_interfaces::prover::Prover;
use crucible_interfaces::Verifier;
use crucible_prover_core::ProverService;
use crucible_vectors::load_catalog;

use crate::paths;

#[derive(Debug, Subcommand)]
pub enum VectorsCommand {
    /// Judge every catalog vector through the mock tier.
    Run {
        /// Restrict judging to one operation circuit.
        #[arg(long, value_name = "OP")]
        op: Option<String>,
        /// Catalog directory (defaults to `<repo>/test-vectors`).
        #[arg(long, value_name = "DIR")]
        catalog: Option<PathBuf>,
    },
}

pub fn run(command: VectorsCommand) -> Result<(), String> {
    match command {
        VectorsCommand::Run { op, catalog } => run_catalog(op.as_deref(), catalog),
    }
}

fn run_catalog(op: Option<&str>, catalog: Option<PathBuf>) -> Result<(), String> {
    let root = catalog.unwrap_or_else(paths::default_catalog_root);
    let vectors = load_catalog(&root)
        .map_err(|e| format!("cannot load catalog from `{}`: {e}", root.display()))?;
    let vectors: Vec<_> = match op {
        Some(name) => {
            let selected: Vec<_> = vectors
                .into_iter()
                .filter(|v| v.operation.as_str() == name)
                .collect();
            if selected.is_empty() {
                return Err(format!(
                    "no vectors found for operation `{name}` in `{}`",
                    root.display()
                ));
            }
            selected
        }
        None => vectors,
    };

    let mut service = ProverService::new(concat!("crucible-prover/", env!("CARGO_PKG_VERSION")));
    service.register_provider(Box::new(crucible_mock::MockProver::new()));
    let verifier = crucible_mock::MockVerifier::new();
    service.with_local_verifier(Box::new(verifier.clone()));

    let mut failed = Vec::new();

    for vector in &vectors {
        let result = judge(&service, &verifier, vector);
        match result {
            Ok(()) => {
                println!("ok   {} ({}/{})", vector.id, vector.operation, vector.category);
            }
            Err(reason) => {
                println!("FAIL {} ({}/{}): {reason}", vector.id, vector.operation, vector.category);
                failed.push((vector.id.clone(), reason));
            }
        }
    }

    if failed.is_empty() {
        let valid = vectors.iter().filter(|v| v.expect_verification).count();
        println!(
            "{} vector(s) judged ({} valid, {} rejecting), all green through the mock tier",
            vectors.len(),
            valid,
            vectors.len() - valid
        );
        println!(
            "circuit tier: cargo test -p crucible-tests --test vectors (requires nargo on PATH)"
        );
        Ok(())
    } else {
        Err(format!(
            "{} of {} vector(s) failed the mock tier",
            failed.len(),
            vectors.len()
        ))
    }
}

/// Judges one vector: valid vectors must round-trip and verify; rejecting
/// vectors must at least be expressible as provable requests.
fn judge(
    service: &ProverService,
    verifier: &crucible_mock::MockVerifier,
    vector: &crucible_vectors::TestVector,
) -> Result<(), String> {
    let request = vector.to_request();
    request
        .validate()
        .map_err(|e| format!("request is structurally invalid: {e}"))?;

    let response = service
        .prove(&request)
        .map_err(|e| format!("mock could not prove the well-formed request: {e}"))?;

    if vector.expect_verification {
        let outcome = verifier
            .verify(&crucible_interfaces::VerificationRequest::from_response(&response))
            .map_err(|e| format!("verifier failed to run: {e}"))?;
        if !outcome.verified {
            return Err(format!(
                "proof did not verify: {}",
                outcome
                    .failure
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            ));
        }
    }
    Ok(())
}