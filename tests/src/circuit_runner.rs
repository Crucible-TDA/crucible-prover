//! Circuit-tier execution for the test-vector catalog.
//!
//! The mock tier (see [`crate::MockStack`]) can only prove that a request is
//! *well-formed*; it is semantically blind. Judging whether a vector's
//! witness actually satisfies the Noir circuit requires the real toolchain:
//! `nargo execute` solves the witness from a `Prover.toml`, and an
//! unsatisfiable witness (wrong owner, overdraw, opening mismatch) fails to
//! solve.
//!
//! This module copies a circuit package (plus its `circuit_lib` dependency
//! and compiled artifact) into a scratch directory, writes the vector's
//! witness as `Prover.toml`, runs `nargo execute`, and reports whether the
//! witness solved and what public outputs the circuit reported. It requires
//! the `nargo` binary on `PATH`; callers gate on
//! [`NoirToolchain::is_available`].

use std::path::{Path, PathBuf};

use crucible_noir::NoirToolchain;
use crucible_noir::compiler::{CapturedExecute, ReportedOutputs};

use crate::vectors::TestVector;

/// Locates the circuits workspace relative to this crate's manifest.
pub fn circuits_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../circuits")
}

/// A scratch copy of one circuit package, ready to execute witnesses.
pub struct CircuitScratch {
    /// The package directory (`<temp>/<op>`).
    pub package_dir: PathBuf,
    _temp: tempfile::TempDir,
}

impl CircuitScratch {
    /// Copies `<op>`'s package and its `circuit_lib` dependency into a fresh
    /// temp directory. When the repo has a precompiled artifact for the op,
    /// it is copied along so `nargo execute` skips recompilation.
    pub fn prepare(op: &str) -> Result<CircuitScratch, String> {
        let circuits = circuits_root();
        let package = circuits.join(op);
        if !package.join("Nargo.toml").is_file() {
            return Err(format!(
                "circuit package `{op}` not found under `{}`",
                circuits.display()
            ));
        }
        let temp = tempfile::tempdir().map_err(|e| format!("cannot create scratch dir: {e}"))?;
        let package_dir = temp.path().join(op);
        let lib_dir = temp.path().join("lib");

        copy_tree(&package, &package_dir)?;
        copy_tree(&circuits.join("lib"), &lib_dir)?;

        // Precompiled artifact short-circuits compilation; harmless if absent
        // (nargo will compile the scratch copy, needing only the sources above).
        let artifact = circuits.join("target").join(format!("{op}.json"));
        if artifact.is_file() {
            std::fs::create_dir_all(package_dir.join("target"))
                .map_err(|e| format!("cannot create target dir: {e}"))?;
            std::fs::copy(
                &artifact,
                package_dir.join("target").join(format!("{op}.json")),
            )
            .map_err(|e| format!("cannot copy artifact `{}`: {e}", artifact.display()))?;
        }
        Ok(CircuitScratch {
            package_dir,
            _temp: temp,
        })
    }

    /// Runs `nargo execute` against `Prover.toml` inside the package dir,
    /// returning the captured run.
    pub fn execute(&self, toolchain: &NoirToolchain) -> Result<CapturedExecute, String> {
        let toml = self.package_dir.join("Prover.toml");
        if !toml.is_file() {
            return Err(format!(
                "missing Prover.toml in {}",
                self.package_dir.display()
            ));
        }
        toolchain
            .execute_captured(&self.package_dir, "Prover", "vector")
            .map_err(|e| format!("nargo execute failed: {e}"))
    }
}

/// Writes a vector's witness as `Prover.toml` in the scratch package dir.
///
/// Prover.toml contains **every** circuit input: private witness values and
/// public inputs alike (the pub/private split lives in the circuit, not the
/// input file). Values are written as `0x`-prefixed quoted hex so no decimal
/// ambiguity can arise.
pub fn write_prover_toml(scratch: &CircuitScratch, vector: &TestVector) -> Result<(), String> {
    let mut lines = Vec::new();
    for (name, hex) in &vector.witness.private {
        lines.push(format!("{name} = \"0x{hex}\""));
    }
    for (name, value) in vector.witness.public.iter() {
        lines.push(format!("{name} = \"0x{}\"", value.as_hex()));
    }
    // Deterministic order: private names sorted, public entries in bag order.
    let toml = lines.join("\n") + "\n";
    std::fs::write(scratch.package_dir.join("Prover.toml"), toml)
        .map_err(|e| format!("cannot write Prover.toml: {e}"))
}

/// The verdict of executing one vector against the real circuit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitVerdict {
    /// Whether `nargo execute` solved the witness.
    pub solved: bool,
    /// The reported public outputs, when solved (raw `0x…` hex).
    pub reported: ReportedOutputs,
}

/// Executes `vector`'s witness against the real circuit package `op`.
///
/// The caller is responsible for ensuring nargo is available and for
/// preparing the scratch once per op (execution is the slow part).
pub fn execute_vector(
    toolchain: &NoirToolchain,
    scratch: &CircuitScratch,
    vector: &TestVector,
) -> Result<CircuitVerdict, String> {
    write_prover_toml(scratch, vector)?;
    let captured = scratch.execute(toolchain)?;
    Ok(CircuitVerdict {
        solved: captured.solved,
        reported: ReportedOutputs::parse_stdout(&captured.stdout),
    })
}

/// Compares the circuit's reported outputs against a vector's expected
/// outputs positionally: `expected_public_outputs` entries are in circuit
/// return order, and the circuit reports its return tuple in the same order.
pub fn assert_outputs_match(vector: &TestVector, reported: &ReportedOutputs) -> Result<(), String> {
    let expected: Vec<_> = vector.expected_public_outputs.iter().collect();
    if expected.len() != reported.values.len() {
        return Err(format!(
            "vector `{}`: circuit reported {} outputs, expected {} ({}): reported {:?}",
            vector.id,
            reported.values.len(),
            expected.len(),
            expected
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", "),
            reported.values
        ));
    }
    for ((name, expected_value), reported_hex) in expected.iter().zip(reported.values.iter()) {
        let reported_value =
            crucible_interfaces::FieldValue::from_hex(reported_hex).map_err(|e| {
                format!(
                    "vector `{}`: circuit output `{reported_hex}`: {e}",
                    vector.id
                )
            })?;
        if reported_value != **expected_value {
            return Err(format!(
                "vector `{}`: output `{name}` mismatch: expected {}, circuit reported {}",
                vector.id,
                expected_value.as_hex(),
                reported_value.as_hex()
            ));
        }
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    if from.is_dir() {
        std::fs::create_dir_all(to)
            .map_err(|e| format!("cannot create `{}`: {e}", to.display()))?;
        for entry in
            std::fs::read_dir(from).map_err(|e| format!("cannot read `{}`: {e}", from.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else if from.is_file() {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create `{}`: {e}", parent.display()))?;
        }
        std::fs::copy(from, to).map_err(|e| {
            format!(
                "cannot copy `{}` -> `{}`: {e}",
                from.display(),
                to.display()
            )
        })?;
    }
    Ok(())
}
