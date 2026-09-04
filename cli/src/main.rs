//! `crucible-prover` — the command-line face of the proving engine.
//!
//! The CLI is pure orchestration: every command shells out to the library
//! crates (circuit compilation through the `noir` adapter, proving through
//! [`ProverService`], verification through the registered verifiers, catalog
//! judging through `crucible-vectors`). No proving logic lives here.

mod commands;
mod paths;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::commands::{artifacts, circuits, prove, vectors, verify};

#[derive(Debug, Parser)]
#[command(
    name = "crucible-prover",
    version,
    about = "Zero-knowledge proving infrastructure for Stellar Confidential Tokens"
)]
struct Cli {
    /// Path to the circuits workspace (defaults to `<repo>/circuits`).
    #[arg(long, global = true, value_name = "DIR")]
    circuits: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect, check, and compile the Noir circuit workspace.
    Circuits {
        #[command(subcommand)]
        command: circuits::CircuitsCommand,
    },
    /// Pin, verify, and inspect the compiled circuit artifacts proving uses.
    Artifacts {
        #[command(subcommand)]
        command: artifacts::ArtifactsCommand,
    },
    /// Build a witness from a test vector and produce a proof envelope.
    Prove(prove::ProveArgs),
    /// Verify a proof envelope against the matching backend verifier.
    Verify(verify::VerifyArgs),
    /// Judge the test-vector catalog through the mock tier.
    Vectors {
        #[command(subcommand)]
        command: vectors::VectorsCommand,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let circuits = cli
        .circuits
        .clone()
        .unwrap_or_else(paths::default_circuits_root);
    match run(cli.command, &circuits) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command, circuits: &std::path::Path) -> Result<(), String> {
    match command {
        Command::Circuits { command } => circuits::run(command, circuits),
        Command::Artifacts { command } => artifacts::run(command, circuits),
        Command::Prove(args) => prove::run(args, circuits),
        Command::Verify(args) => verify::run(args),
        Command::Vectors { command } => vectors::run(command),
    }
}
