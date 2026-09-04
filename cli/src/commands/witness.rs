//! `crucible-prover witness build` — assemble and emit a circuit witness.
//!
//! Proving assembles a witness implicitly inside the service; this command
//! exposes the same assembly step directly so operators can build a
//! `Prover.toml` from a test vector (or any well-formed request shape) and
//! inspect or hand it to a toolchain. It is also the debugging surface for
//! witness problems: what the circuit will see as public context, and which
//! private names it must receive.
//!
//! Privacy rule: private values leave memory only through
//! `crucible-witness`'s restricted encoder (0600 files). The summary printed
//! here shows public values and private *names* — never private values.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use crucible_vectors::TestVector;
use crucible_witness::{WitnessData, encoder};

#[derive(Debug, Subcommand)]
pub enum WitnessCommand {
    /// Assemble a circuit witness from a test vector.
    ///
    /// Prints a redacted summary (public values, private *names*); with
    /// `--out` it also writes the Noir `Prover.toml` layout through the
    /// restricted encoder (0600 on Unix).
    Build(BuildArgs),
}

pub fn run(command: WitnessCommand) -> Result<(), String> {
    match command {
        WitnessCommand::Build(args) => run_build(args),
    }
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Operation circuit the witness is for (must match the vector).
    pub op: String,
    /// Test-vector JSON file describing the witness (see
    /// `schemas/test-vector.schema.json`).
    #[arg(long, value_name = "PATH")]
    pub vector: PathBuf,
    /// Write the witness as a Noir `Prover.toml` at this path (0600).
    ///
    /// Without this flag the command only prints a redacted summary.
    #[arg(long, value_name = "PATH")]
    pub out: Option<PathBuf>,
}

fn run_build(args: BuildArgs) -> Result<(), String> {
    let vector = TestVector::load(&args.vector)
        .map_err(|e| format!("cannot load vector `{}`: {e}", args.vector.display()))?;
    if vector.operation.as_str() != args.op {
        return Err(format!(
            "operation `{}` does not match vector `{}` (operation {})",
            args.op, vector.id, vector.operation
        ));
    }

    let request = vector.to_request();
    request
        .validate()
        .map_err(|e| format!("request is structurally invalid: {e}"))?;
    let witness = WitnessData::from_request(&request);

    match &args.out {
        Some(path) => {
            encoder::write_prover_toml(&witness, path)
                .map_err(|e| format!("cannot write witness `{}`: {e}", path.display()))?;
            println!("wrote {} (private values, 0600)", path.display());
        }
        None => {
            println!("summary only; pass --out <path> to write a Prover.toml");
        }
    }

    print_summary(&vector, &witness);
    Ok(())
}

/// Prints public values and private *names* — never private values.
fn print_summary(vector: &TestVector, witness: &WitnessData) {
    println!("vector      {} ({})", vector.id, vector.category);
    println!("operation   {}", witness.operation());
    println!("public      {} value(s)", witness.public_inputs().len());
    for (name, value) in witness.public_inputs().iter() {
        println!("  {name} = {}", value.as_hex());
    }
    println!("private     {} value(s)", witness.private_count());
    for name in witness.private().names() {
        println!("  {name} (redacted)");
    }
    if let Some(state) = &vector.state_reference {
        println!("state       root {} seq {}", state.root, state.sequence);
    }
}
