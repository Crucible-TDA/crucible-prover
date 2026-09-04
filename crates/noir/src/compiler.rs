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
}
