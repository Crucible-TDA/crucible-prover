//! `crucible-prover artifacts` — pin, verify, and inspect the compiled
//! circuit artifacts the proving path consumes.
//!
//! Proving must never run against ad-hoc bytecode: a swapped or tampered
//! circuit is exactly the attack the threat model calls the *artifact
//! swapper* (A4). The provider therefore proves from a **pinned artifact
//! root** (`<repo>/artifacts/circuits/<op>/`), where every artifact sits
//! next to a `manifest.json` declaring each file's SHA-256. This command
//! group creates those roots (`generate`) and verifies them (`check`)
//! through the `crucible-artifacts` loader — the same strict loader the
//! provider runs before touching any bytecode.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use crucible_artifacts::{ArtifactLoader, ArtifactManifest, MANIFEST_SCHEMA_VERSION, ManifestFile};
use crucible_interfaces::{ArtifactChecksum, BackendId, CircuitId, Version};

use crate::commands::circuits::OPERATIONS;
use crate::paths;

#[derive(Debug, Subcommand)]
pub enum ArtifactsCommand {
    /// Verify every pinned circuit artifact against its manifest.
    ///
    /// Loads each `<root>/<op>/` directory with the strict artifact loader:
    /// every declared file must match its SHA-256 and no undeclared file may
    /// be present. Exits non-zero listing every problem.
    Check {
        /// Pinned artifact root (defaults to `<repo>/artifacts/circuits`).
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
    /// Re-pin circuit artifacts from the current compiled bytecode.
    ///
    /// Copies `<circuits>/target/<op>.json` into `<root>/<op>/<op>.json`
    /// next to a generated `manifest.json`. Requires compiled bytecode (run
    /// `crucible-prover circuits compile` first). Deterministic: identical
    /// bytecode reproduces byte-identical manifests.
    Generate {
        /// Circuit to re-pin; defaults to all five.
        #[arg(value_name = "OP")]
        op: Option<String>,
        /// Pinned artifact root (defaults to `<repo>/artifacts/circuits`).
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
    /// Inspect one pinned artifact: manifest provenance and per-file
    /// checksums, verified through the strict loader.
    ///
    /// Answers the four provenance questions of any proof produced from the
    /// artifact (circuit, versions, backend, verification key) and shows
    /// each declared file's SHA-256 and byte size. Runs the same strict
    /// integrity verification as `check`, so a tampered artifact is reported
    /// here too — with the manifest still printed for diagnostics.
    Inspect {
        /// Circuit to inspect; defaults to all five.
        #[arg(value_name = "OP")]
        op: Option<String>,
        /// Pinned artifact root (defaults to `<repo>/artifacts/circuits`).
        #[arg(long, value_name = "DIR")]
        root: Option<PathBuf>,
    },
}

/// The manifest filename the loader expects inside an artifact directory.
const MANIFEST_FILENAME: &str = "manifest.json";

pub fn run(command: ArtifactsCommand, circuits: &Path) -> Result<(), String> {
    match command {
        ArtifactsCommand::Check { root } => check(root),
        ArtifactsCommand::Generate { op, root } => generate(circuits, op.as_deref(), root),
        ArtifactsCommand::Inspect { op, root } => inspect(op.as_deref(), root),
    }
}

fn artifact_root(explicit: &Option<PathBuf>) -> PathBuf {
    explicit
        .clone()
        .unwrap_or_else(paths::default_artifact_root)
}

fn check(explicit_root: Option<PathBuf>) -> Result<(), String> {
    let root = artifact_root(&explicit_root);
    let loader = ArtifactLoader::new();
    let mut problems = Vec::new();

    for op in OPERATIONS {
        let dir = root.join(op);
        match loader.load(&dir) {
            Ok(artifact) => {
                println!(
                    "ok   {op}: manifest v{} — {} file(s) verified",
                    artifact.manifest.manifest_version,
                    artifact.manifest.files.len(),
                );
            }
            Err(e) => {
                println!("FAIL {op}: {e}");
                problems.push(format!("{op}: {e}"));
            }
        }
    }

    if problems.is_empty() {
        println!("all {} pinned artifacts verified", OPERATIONS.len());
        Ok(())
    } else {
        Err(format!(
            "{} of {} pinned artifacts failed integrity verification (run \
             `crucible-prover artifacts generate` to re-pin from current \
             bytecode)",
            problems.len(),
            OPERATIONS.len()
        ))
    }
}

fn generate(
    circuits: &Path,
    op: Option<&str>,
    explicit_root: Option<PathBuf>,
) -> Result<(), String> {
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
    let root = artifact_root(&explicit_root);

    for op in selected {
        let bytecode_path = paths::artifact_path(circuits, op);
        let bytes = std::fs::read(&bytecode_path).map_err(|e| {
            format!(
                "cannot read compiled bytecode `{}` for `{op}` (run \
                 `crucible-prover circuits compile` first): {e}",
                bytecode_path.display()
            )
        })?;

        let dir = root.join(op);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("cannot create `{}`: {e}", dir.display()))?;

        let checksum = ArtifactChecksum::from_bytes(&bytes);
        let manifest = ArtifactManifest {
            manifest_version: MANIFEST_SCHEMA_VERSION,
            circuit: CircuitId::new(op.to_owned())
                .map_err(|e| format!("cannot form circuit id: {e}"))?,
            circuit_version: Version::v0_1(),
            artifact_version: Version::v0_1(),
            backend: BackendId::new(BackendId::ULTRAHONK)
                .map_err(|e| format!("cannot form backend id: {e}"))?,
            verification_key_id: None,
            files: vec![ManifestFile {
                path: format!("{op}.json"),
                sha256: checksum.clone(),
                kind: Some("acir".to_owned()),
            }],
            backend_metadata: BTreeMap::from([(
                "generated_by".to_owned(),
                format!("crucible-prover/{}", env!("CARGO_PKG_VERSION")),
            )]),
        };

        let artifact_file = dir.join(format!("{op}.json"));
        std::fs::write(&artifact_file, &bytes)
            .map_err(|e| format!("cannot write `{}`: {e}", artifact_file.display()))?;
        let manifest_file = dir.join(MANIFEST_FILENAME);
        std::fs::write(&manifest_file, manifest.to_canonical_json())
            .map_err(|e| format!("cannot write `{}`: {e}", manifest_file.display()))?;

        println!(
            "pinned {op}: {} ({}) + {}",
            artifact_file.display(),
            checksum.as_hex(),
            manifest_file.display(),
        );
    }
    Ok(())
}

/// Resolves the operations to inspect, validating explicit names.
fn selected_ops(op: Option<&str>) -> Result<Vec<String>, String> {
    match op {
        Some(name) => {
            if !OPERATIONS.contains(&name) {
                return Err(format!(
                    "unknown circuit `{name}` (expected one of {})",
                    OPERATIONS.join(", ")
                ));
            }
            Ok(vec![name.to_owned()])
        }
        None => Ok(OPERATIONS.iter().map(|s| s.to_string()).collect()),
    }
}

fn inspect(op: Option<&str>, explicit_root: Option<PathBuf>) -> Result<(), String> {
    let selected = selected_ops(op)?;
    let root = artifact_root(&explicit_root);
    let loader = ArtifactLoader::new();
    let mut problems = Vec::new();

    for op in &selected {
        let dir = root.join(op);
        println!("{op}:");
        match loader.load(&dir) {
            Ok(artifact) => {
                println!(
                    "  integrity   ok (manifest v{}; {} file(s) verified)",
                    artifact.manifest.manifest_version,
                    artifact.manifest.files.len(),
                );
                println!(
                    "  circuit     {} v{}",
                    artifact.manifest.circuit, artifact.manifest.circuit_version
                );
                println!("  artifact    v{}", artifact.manifest.artifact_version);
                println!("  backend     {}", artifact.manifest.backend);
                println!(
                    "  vk id       {}",
                    artifact
                        .manifest
                        .verification_key_id
                        .as_ref()
                        .map(|id| id.as_str().to_owned())
                        .unwrap_or_else(|| "(none)".to_owned())
                );
                for (key, value) in &artifact.manifest.backend_metadata {
                    println!("  metadata    {key}={value}");
                }
                println!("  files");
                for declared in &artifact.manifest.files {
                    let size = artifact
                        .files
                        .iter()
                        .find(|(path, _)| path == &declared.path)
                        .map(|(_, bytes)| bytes.len())
                        .unwrap_or(0);
                    let kind = declared.kind.as_deref().unwrap_or("file");
                    println!(
                        "    {:<20} {:>8} B  kind={:<15} sha256 {}",
                        declared.path,
                        size,
                        kind,
                        declared.sha256.as_hex()
                    );
                }
            }
            Err(e) => {
                // Diagnostics: print the raw manifest too, so a failing
                // artifact can be inspected even when the loader refuses it.
                println!("  integrity   FAIL: {e}");
                let manifest_path = dir.join(MANIFEST_FILENAME);
                if let Ok(text) = std::fs::read_to_string(&manifest_path) {
                    println!("  manifest    {text}");
                }
                problems.push(format!("{op}: {e}"));
            }
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} pinned artifact(s) failed integrity verification (run \
             `crucible-prover artifacts generate` to re-pin from current \
             bytecode)",
            problems.len(),
        ))
    }
}
