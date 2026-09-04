//! Prove one operation and verify it locally (mock backend).
//!
//! Usage: prove-verify <op> [--vector <path>]
//!
//! The vector defaults to the operation's committed valid fixture. Exit
//! code is zero only when the proof passes its own round trip.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crucible_interfaces::prover::Prover;
use crucible_mock::{MockProver, MockVerifier};
use crucible_prover_core::ProverService;
use crucible_vectors::TestVector;

fn catalog_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors")
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let op = match args.next() {
        Some(op) => op,
        None => {
            eprintln!("usage: prove-verify <op> [--vector <path>]");
            return ExitCode::FAILURE;
        }
    };
    let mut vector_path: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        if arg == "--vector" {
            vector_path = args.next().map(PathBuf::from);
        } else {
            eprintln!("unknown argument `{arg}` (expected --vector <path>)");
            return ExitCode::FAILURE;
        }
    }
    let path = match vector_path {
        Some(path) => path,
        None => catalog_root()
            .join(&op)
            .join("valid")
            .join(format!("{op}-valid-001.json")),
    };

    let vector = match TestVector::load(&path) {
        Ok(v) if v.operation.as_str() == op => v,
        Ok(v) => {
            eprintln!(
                "operation `{op}` does not match vector `{}` (operation {})",
                v.id, v.operation
            );
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("cannot load vector `{}`: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let request = vector.to_request();
    if let Err(e) = request.validate() {
        eprintln!("request is structurally invalid: {e}");
        return ExitCode::FAILURE;
    }

    let mut service = ProverService::new("crucible-examples/0.1.0");
    service.register_provider(Box::new(MockProver::new()));
    service.with_local_verifier(Box::new(MockVerifier::new()));

    match service.prove_and_verify(&request) {
        Ok((response, outcome)) if outcome.verified => {
            println!(
                "verified: {} v{} — {} public word(s), {} proof bytes, backend {}",
                response.circuit,
                response.circuit_version,
                response.public_outputs.len(),
                response.proof.bytes.len(),
                response.backend
            );
            ExitCode::SUCCESS
        }
        Ok((_, outcome)) => {
            eprintln!(
                "proof rejected: {}",
                outcome
                    .failure
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            );
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("proving failed: {e}");
            ExitCode::FAILURE
        }
    }
}
