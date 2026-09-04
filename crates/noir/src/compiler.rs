//! Driving `nargo compile` and `nargo execute`.
//!
//! All process execution for the Noir toolchain lives here, behind
//! [`NoirToolchain`]. Compilation produces ACIR artifacts in `target/`;
//! execution solves a witness from a `Prover.toml` and writes the witness to
//! `target/<name>.gz`, which the `ultrahonk` adapter later consumes for
//! proof generation.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::errors::NoirError;
use crate::toolchain::NoirToolchain;

/// Output of a successful `nargo compile` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOutput {
    /// Path of the compiled artifact JSON (one per package).
    pub artifact_path: PathBuf,
}

/// Output of a successful `nargo execute` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteOutput {
    /// Path of the solved witness file (`target/<name>.gz`).
    pub witness_path: PathBuf,
}

/// Output of an `nargo execute` run whose stdout and exit status were
/// captured.
///
/// Capturing is opt-in (see [`NoirToolchain::execute_captured`]) for callers
/// that need the printed `Circuit output` line (e.g. the vector runner
/// comparing solved return values against fixtures) or that treat
/// "did not solve" as a first-class outcome (invalid vectors). The captured
/// text is returned to the caller, never embedded into errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedExecute {
    /// Whether the witness solved (exit code 0 and a witness file produced).
    pub solved: bool,
    /// The process exit code, when nargo reported one.
    pub exit_code: Option<i32>,
    /// Path of the solved witness file, when one was produced.
    pub witness_path: Option<PathBuf>,
    /// Captured stdout from the run (includes the `Circuit output:` line).
    pub stdout: String,
}

/// The public values `nargo execute` reports for a solved circuit.
///
/// `nargo execute` prints the circuit's return tuple as `Circuit output:
/// (0x…, 0x…)`. Each value is kept as the raw reported hex so callers can
/// canonicalize it with their value model.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReportedOutputs {
    /// Reported values in return order, `0x`-prefixed lowercase hex.
    pub values: Vec<String>,
}

impl ReportedOutputs {
    /// Parses the `Circuit output:` line out of captured `nargo execute`
    /// stdout, returning the reported values in order.
    ///
    /// Circuits without a public return (e.g. `register`) print no output
    /// line; that yields an empty [`ReportedOutputs`] rather than an error.
    pub fn parse_stdout(stdout: &str) -> ReportedOutputs {
        let line = stdout
            .lines()
            .find(|l| l.contains("Circuit output:"))
            .map(str::to_owned);
        let Some(line) = line else {
            return ReportedOutputs::default();
        };
        let values = extract_parenthesized_hex(&line);
        ReportedOutputs { values }
    }
}

/// Extracts `0x…` tokens from a parenthesized list like
/// `Circuit output: (0x1, 0x2, 0x3)`. Also tolerates a single bare value
/// without parentheses for circuits returning one field.
fn extract_parenthesized_hex(line: &str) -> Vec<String> {
    let open = line.find('(').unwrap_or(0);
    let close = line.rfind(')');
    let body = match close {
        Some(end) => &line[open + 1..end],
        None => &line[open..],
    };
    body.split([',', ' ', '\t'])
        .filter(|t| t.starts_with("0x") || t.starts_with("0X"))
        .map(|t| t.to_ascii_lowercase())
        .collect()
}

impl NoirToolchain {
    /// Runs `nargo compile` in `project_dir`.
    ///
    /// `package` optionally selects one package of a workspace. The compiled
    /// artifact is expected at `target/<package>.json`.
    pub fn compile(&self, project_dir: &Path, package: &str) -> Result<CompileOutput, NoirError> {
        self.check_version()?;
        let status = Command::new(self.binary())
            .current_dir(project_dir)
            .args(["compile", "--silence-warnings", "--package", package])
            .status()
            .map_err(|e| NoirError::Io {
                path: project_dir.display().to_string(),
                reason: e.to_string(),
            })?;
        if !status.success() {
            return Err(NoirError::CommandFailed {
                command: "compile".to_owned(),
                status: status.code().unwrap_or(-1),
            });
        }
        let artifact_path = project_dir.join("target").join(format!("{package}.json"));
        if !artifact_path.is_file() {
            return Err(NoirError::ExpectedOutput {
                path: artifact_path.display().to_string(),
                command: "compile".to_owned(),
            });
        }
        Ok(CompileOutput { artifact_path })
    }

    /// Runs `nargo execute` in `project_dir` to solve a witness from
    /// `Prover.toml`.
    ///
    /// `witness_name` names the output witness file (`target/<witness_name>.gz`).
    pub fn execute(
        &self,
        project_dir: &Path,
        prover_toml: &str,
        witness_name: &str,
    ) -> Result<ExecuteOutput, NoirError> {
        self.check_version()?;
        let status = Command::new(self.binary())
            .current_dir(project_dir)
            .args(["execute", "-p", prover_toml, witness_name])
            .status()
            .map_err(|e| NoirError::Io {
                path: project_dir.display().to_string(),
                reason: e.to_string(),
            })?;
        if !status.success() {
            return Err(NoirError::CommandFailed {
                command: "execute".to_owned(),
                status: status.code().unwrap_or(-1),
            });
        }
        let witness_path = project_dir
            .join("target")
            .join(format!("{witness_name}.gz"));
        if !witness_path.is_file() {
            return Err(NoirError::ExpectedOutput {
                path: witness_path.display().to_string(),
                command: "execute".to_owned(),
            });
        }
        Ok(ExecuteOutput { witness_path })
    }

    /// Runs `nargo execute`, capturing stdout and the exit status.
    ///
    /// This is the vector runner's circuit oracle: valid vectors must solve
    /// and their reported outputs must match the fixture; invalid vectors
    /// must *not* solve. Unlike [`NoirToolchain::execute`], a non-zero exit
    /// is not an error — it is recorded on the returned [`CapturedExecute`]
    /// so callers with "must fail" expectations can assert on it directly.
    pub fn execute_captured(
        &self,
        project_dir: &Path,
        prover_toml: &str,
        witness_name: &str,
    ) -> Result<CapturedExecute, NoirError> {
        self.check_version()?;
        // Remove any witness left by a previous run so `solved` below is
        // authoritative for *this* execution.
        let witness_path = project_dir
            .join("target")
            .join(format!("{witness_name}.gz"));
        let _ = std::fs::remove_file(&witness_path);

        let output = Command::new(self.binary())
            .current_dir(project_dir)
            .args(["execute", "-p", prover_toml, witness_name])
            .output()
            .map_err(|e| NoirError::Io {
                path: project_dir.display().to_string(),
                reason: e.to_string(),
            })?;
        let solved = output.status.success();
        if solved && !witness_path.is_file() {
            return Err(NoirError::ExpectedOutput {
                path: witness_path.display().to_string(),
                command: "execute".to_owned(),
            });
        }
        Ok(CapturedExecute {
            solved,
            exit_code: output.status.code(),
            witness_path: solved.then_some(witness_path),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("Nargo.toml"),
            "[package]\nauthors = [\"crucible-test\"]\ncompiler_version = \">=0.1.0\"\nname = \"demo\"\ntype = \"bin\"\n\n[dependencies]\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("src").join("main.nr"),
            "fn main(x: pub Field, y: Field) -> pub Field {\n    x + y\n}\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn compiles_a_minimal_circuit_when_nargo_is_available() {
        if !NoirToolchain::is_available() {
            eprintln!("skipping: nargo not installed");
            return;
        }
        let dir = scratch_project();
        let toolchain = NoirToolchain::locate().unwrap();
        let output = toolchain.compile(dir.path(), "demo").unwrap();
        assert!(output.artifact_path.is_file());
        let artifact = crate::artifact::CompiledArtifact::from_file(&output.artifact_path).unwrap();
        assert_eq!(artifact.public_parameter_names(), vec!["x"]);
    }

    #[test]
    fn executes_a_witness_when_nargo_is_available() {
        if !NoirToolchain::is_available() {
            eprintln!("skipping: nargo not installed");
            return;
        }
        let dir = scratch_project();
        let toolchain = NoirToolchain::locate().unwrap();
        toolchain.compile(dir.path(), "demo").unwrap();
        std::fs::write(dir.path().join("Prover.toml"), "x = \"3\"\ny = \"4\"\n").unwrap();
        let output = toolchain.execute(dir.path(), "Prover", "w").unwrap();
        assert!(output.witness_path.is_file());
    }

    #[test]
    fn captured_execute_reports_outputs_when_nargo_is_available() {
        if !NoirToolchain::is_available() {
            eprintln!("skipping: nargo not installed");
            return;
        }
        let dir = scratch_project();
        let toolchain = NoirToolchain::locate().unwrap();
        toolchain.compile(dir.path(), "demo").unwrap();
        std::fs::write(dir.path().join("Prover.toml"), "x = \"3\"\ny = \"4\"\n").unwrap();
        let captured = toolchain
            .execute_captured(dir.path(), "Prover", "w")
            .unwrap();
        assert!(captured.solved, "valid witness must solve");
        assert_eq!(captured.exit_code, Some(0));
        assert!(captured.witness_path.is_some());
        let reported = ReportedOutputs::parse_stdout(&captured.stdout);
        // x + y = 7, reported as a field element.
        assert_eq!(reported.values.len(), 1);
        let value = u64::from_str_radix(reported.values[0].trim_start_matches("0x"), 16).unwrap();
        assert_eq!(value, 7);
    }

    #[test]
    fn captured_execute_marks_unsatisfiable_witness_unsolved_when_nargo_is_available() {
        if !NoirToolchain::is_available() {
            eprintln!("skipping: nargo not installed");
            return;
        }
        let dir = scratch_project();
        let toolchain = NoirToolchain::locate().unwrap();
        toolchain.compile(dir.path(), "demo").unwrap();
        // Assert the impossible so the witness cannot solve.
        std::fs::write(dir.path().join("Prover.toml"), "x = \"3\"\ny = \"3\"\n").unwrap();
        std::fs::write(
            dir.path().join("src").join("main.nr"),
            "fn main(x: pub Field, y: Field) -> pub Field {\n    assert(x != y);\n    x + y\n}\n",
        )
        .unwrap();
        toolchain.compile(dir.path(), "demo").unwrap();
        let captured = toolchain
            .execute_captured(dir.path(), "Prover", "w")
            .unwrap();
        assert!(!captured.solved, "unsatisfiable witness must not solve");
        assert_ne!(captured.exit_code, Some(0));
        assert!(captured.witness_path.is_none());
    }

    #[test]
    fn parses_tuple_circuit_output() {
        let stdout = "[demo] Circuit witness successfully solved\n[demo] Witness saved to target/w.gz\n[demo] Circuit output: (0x2a, 0x1b98, 0x2c90)\n";
        let reported = ReportedOutputs::parse_stdout(stdout);
        assert_eq!(reported.values, vec!["0x2a", "0x1b98", "0x2c90"]);
    }

    #[test]
    fn parses_single_value_circuit_output() {
        let stdout = "[demo] Circuit output: (0xdeadbeef)\n";
        let reported = ReportedOutputs::parse_stdout(stdout);
        assert_eq!(reported.values, vec!["0xdeadbeef"]);
    }

    #[test]
    fn empty_stdout_yields_no_outputs() {
        // register-like circuits with no public return print no output line.
        assert!(ReportedOutputs::parse_stdout("").values.is_empty());
        assert!(
            ReportedOutputs::parse_stdout("[demo] solved\n")
                .values
                .is_empty()
        );
    }

    #[test]
    fn ignores_non_hex_tokens_and_upper_case_hex() {
        let stdout = "[demo] Circuit output: (0XAB, 12, 0xCDEF)\n";
        let reported = ReportedOutputs::parse_stdout(stdout);
        assert_eq!(reported.values, vec!["0xab", "0xcdef"]);
    }
}
