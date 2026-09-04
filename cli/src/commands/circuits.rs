//! `crucible-prover circuits` — inspect, check, and compile the Noir
//! circuit workspace.
//!
//! The five operation circuits are the canonical catalog; their packages
//! share names with their circuit ids (`register`, `deposit`, `merge`,
//! `transfer`, `withdraw`) and compile to `target/<package>.json` in the
//! circuits workspace. Compilation runs through the `noir` adapter so
//! process execution stays behind the toolchain boundary.

use std::path::Path;

use clap::Subcommand;
use crucible_noir::NoirToolchain;
use sha2::{Digest, Sha256};

use crate::paths;

/// The five operation circuits, in catalog order.
pub const OPERATIONS: [&str; 5] = ["register", "deposit", "merge", "transfer", "withdraw"];

#[derive(Debug, Subcommand)]
pub enum CircuitsCommand {
    /// List the operation circuits and their compiled artifacts.
    List,
    /// Verify every operation circuit has a compiled, parseable artifact.
    Check,
    /// Compile one operation circuit, or all of them.
    ///
    /// Requires `nargo` on PATH (see `scripts/check-circuits.sh`).
    Compile {
        /// Circuit package to compile; defaults to all five.
        #[arg(value_name = "OP")]
        op: Option<String>,
    },
}

/// SHA-256 of a file, hex-encoded.
fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        std::fs::read(path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(hex::encode(digest))
}

/// True when `path` exists and parses as a JSON document.
fn is_parseable_json(path: &Path) -> bool {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some()
}

pub fn run(command: CircuitsCommand, circuits: &Path) -> Result<(), String> {
    match command {
        CircuitsCommand::List => list(circuits),
        CircuitsCommand::Check => check(circuits),
        CircuitsCommand::Compile { op } => compile(circuits, op.as_deref()),
    }
}

fn list(circuits: &Path) -> Result<(), String> {
    println!(
        "{:<10} {:<18} {:<7} sha256",
        "circuit", "artifact", "status"
    );
    for op in OPERATIONS {
        let artifact = paths::artifact_path(circuits, op);
        if artifact.is_file() {
            let size = std::fs::metadata(&artifact).map(|m| m.len()).unwrap_or(0);
            let digest = sha256_file(&artifact).unwrap_or_else(|e| e);
            println!(
                "{:<10} {:<18} {:<7} {}",
                op,
                artifact.display(),
                format!("{size} B"),
                &digest[..16.min(digest.len())],
            );
        } else {
            println!("{:<10} {:<18} {:<7} -", op, artifact.display(), "missing");
        }
    }
    Ok(())
}

fn check(circuits: &Path) -> Result<(), String> {
    let mut problems = Vec::new();
    for op in OPERATIONS {
        let artifact = paths::artifact_path(circuits, op);
        if !artifact.is_file() {
            problems.push(format!("{op}: missing artifact `{}`", artifact.display()));
        } else if !is_parseable_json(&artifact) {
            problems.push(format!(
                "{op}: artifact `{}` is not valid JSON",
                artifact.display()
            ));
        }
    }
    if problems.is_empty() {
        println!(
            "all {} operation circuits are compiled and parseable",
            OPERATIONS.len()
        );
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("error: {problem}");
        }
        Err(format!(
            "{} circuit artifact problem(s); run `crucible-prover circuits compile`",
            problems.len()
        ))
    }
}

fn compile(circuits: &Path, op: Option<&str>) -> Result<(), String> {
    let selected = match op {
        Some(name) => {
            if !OPERATIONS.contains(&name) {
                return Err(format!(
                    "unknown circuit `{name}` (expected one of {})",
                    OPERATIONS.join(", ")
                ));
            }
            vec![name]
        }
        None => OPERATIONS.to_vec(),
    };

    let toolchain = NoirToolchain::locate()
        .map_err(|e| format!("cannot locate nargo: {e} (see scripts/check-circuits.sh)"))?;
    toolchain
        .check_version()
        .map_err(|e| format!("nargo version check failed: {e}"))?;

    for package in selected {
        let output = toolchain
            .compile(circuits, package)
            .map_err(|e| format!("compiling `{package}` failed: {e}"))?;
        println!("compiled {package}: {}", output.artifact_path.display());
    }
    Ok(())
}
