//! The [`UltraHonkProvider`]: a real [`ProofProvider`] backed by `bb`.
//!
//! This is the concrete provider that turns a [`ProofRequest`] into a real
//! UltraHonk proof:
//!
//! ```text
//! ProofRequest (bags) ──► Prover.toml ──► nargo execute ──► witness
//! witness + bytecode ──► bb prove ──► proof / public inputs / vk
//! ```
//!
//! The request's public and private bags are the circuit's parameters by
//! name (see `interfaces::circuit::expectations`), so the Prover.toml is
//! assembled by [`crucible-witness`]'s encoder — the single escape hatch
//! where secret values leave memory — with restrictive permissions, inside a
//! scratch copy of the circuit package that is deleted when generation
//! returns.
//!
//! # Configuration
//!
//! Proving needs a *pinned, integral* circuit and the `nargo`/`bb`
//! toolchains on PATH. The provider therefore takes an [`UltraHonkConfig`]
//! pointing at the pinned artifact root (default
//! `<circuits>/../artifacts/circuits`, see [`UltraHonkConfig::new`]), at the
//! circuits workspace (used only to solve witnesses from source), and at the
//! [`VkStore`] that verification keys are written into.
//!
//! # Artifact integrity
//!
//! The provider never proves against ad-hoc bytecode. Every artifact is
//! strict-loaded through [`crucible_artifacts::ArtifactLoader`] before a
//! single byte is touched: the manifest must parse, every declared file must
//! match its SHA-256, and no undeclared file may be present. A swapped,
//! tampered, or incomplete artifact fails with
//! [`ProviderError::ArtifactIntegrity`] / [`ProviderError::ArtifactUnavailable`]
//! before any proving work starts.
//!
//! # Privacy
//!
//! Private witness values are written to the scratch `Prover.toml` only
//! through the witness encoder and never appear in errors. All errors carry
//! paths, counts, and machine-readable reasons.

use std::path::PathBuf;

use crucible_artifacts::{ArtifactError, ArtifactLoader};
use crucible_interfaces::circuit::expectations;
use crucible_interfaces::{
    ArtifactChecksum, BackendId, CircuitId, FieldValue, OutputBag, ProofBlob, ProofFormat,
    ProofProvider, ProofRequest, ProofResponse, ProviderError, Version,
};
use crucible_noir::NoirToolchain;

use crate::exec::{ProveOptions, prove};
use crate::store::VkStore;
use crate::toolchain::BbToolchain;
use crate::vk::VerificationKeyIdPolicy;
use crate::{PROOF_FORMAT_TAG, UltraHonkBackend};

/// Configuration for a real UltraHonk provider.
#[derive(Debug, Clone)]
pub struct UltraHonkConfig {
    /// Root of the Noir circuits workspace (contains `<op>/` packages, the
    /// `lib/` dependency, and `target/` with compiled bytecode).
    pub circuits_root: PathBuf,
    /// Root holding the pinned circuit artifacts (`<op>/manifest.json` +
    /// `<op>/<op>.json`), integrity-verified before proving.
    pub artifact_root: PathBuf,
    /// Store that produced verification keys are written into.
    pub vk_store: VkStore,
}

impl UltraHonkConfig {
    /// Creates a configuration.
    ///
    /// The pinned artifact root defaults to `<circuits_root>/../artifacts/circuits`
    /// (the repository's `artifacts/circuits/` for a checkout-based
    /// `circuits_root`); override it with
    /// [`UltraHonkConfig::with_artifact_root`].
    pub fn new(circuits_root: impl Into<PathBuf>, vk_store: VkStore) -> UltraHonkConfig {
        let circuits_root = circuits_root.into();
        let artifact_root = circuits_root.join("..").join("artifacts").join("circuits");
        UltraHonkConfig {
            circuits_root,
            artifact_root,
            vk_store,
        }
    }

    /// Overrides the pinned artifact root this configuration proves from.
    pub fn with_artifact_root(mut self, artifact_root: impl Into<PathBuf>) -> UltraHonkConfig {
        self.artifact_root = artifact_root.into();
        self
    }
}

/// A real UltraHonk [`ProofProvider`] (Noir + Barretenberg).
#[derive(Debug, Clone)]
pub struct UltraHonkProvider {
    config: UltraHonkConfig,
}

impl UltraHonkProvider {
    /// Creates a provider that proves against the configured circuits
    /// workspace and writes verification keys into its store.
    pub fn new(config: UltraHonkConfig) -> UltraHonkProvider {
        UltraHonkProvider { config }
    }

    /// The circuit workspace root.
    pub fn circuits_root(&self) -> &std::path::Path {
        &self.config.circuits_root
    }

    /// The verification-key store backing this provider.
    pub fn vk_store(&self) -> &VkStore {
        &self.config.vk_store
    }
}

impl ProofProvider for UltraHonkProvider {
    fn backend(&self) -> BackendId {
        UltraHonkBackend::id()
    }

    fn supports(&self, circuit: &CircuitId, version: &Version) -> bool {
        UltraHonkBackend::supports(circuit, version)
    }

    fn generate(&self, request: &ProofRequest) -> Result<ProofResponse, ProviderError> {
        request.validate()?;
        if request.backend != self.backend() {
            return Err(ProviderError::ProofGeneration {
                backend: request.backend.to_string(),
                reason: "request targets a different backend than the ultrahonk provider"
                    .to_owned(),
            });
        }
        if !self.supports(&request.circuit, &request.circuit_version) {
            return Err(ProviderError::UnsupportedCircuit {
                backend: request.backend.to_string(),
                circuit: request.circuit.clone(),
                version: request.circuit_version,
            });
        }

        // Toolchains must be present and version-gated.
        let bb = match BbToolchain::locate() {
            Ok(toolchain) => toolchain,
            Err(_) => {
                return Err(ProviderError::BackendUnavailable {
                    backend: BackendId::ULTRAHONK.to_owned(),
                    reason: "bb binary not found on PATH (see scripts/check-bb.sh)".to_owned(),
                });
            }
        };
        let nargo = match NoirToolchain::locate() {
            Ok(toolchain) => toolchain,
            Err(_) => {
                return Err(ProviderError::BackendUnavailable {
                    backend: BackendId::ULTRAHONK.to_owned(),
                    reason: "nargo binary not found on PATH (see scripts/check-circuits.sh)"
                        .to_owned(),
                });
            }
        };
        bb.check_version()
            .map_err(|e| ProviderError::BackendUnavailable {
                backend: BackendId::ULTRAHONK.to_owned(),
                reason: e.to_string(),
            })?;
        nargo
            .check_version()
            .map_err(|e| ProviderError::BackendUnavailable {
                backend: BackendId::ULTRAHONK.to_owned(),
                reason: e.to_string(),
            })?;

        // The compiled artifact must be present *and integral*: it is
        // strict-loaded against its manifest (every declared file must match
        // its SHA-256, no undeclared file may be present) before a single
        // byte is touched. Its SHA-256 becomes both the artifact checksum
        // and the VK id discriminator.
        let artifact_dir = self.config.artifact_root.join(request.circuit.as_str());
        let loaded = ArtifactLoader::new().load(&artifact_dir).map_err(|e| {
            let circuit = request.circuit.clone();
            let version = request.circuit_version;
            match e {
                ArtifactError::ReadFailure { .. } | ArtifactError::MissingFile { .. } => {
                    ProviderError::ArtifactUnavailable {
                        backend: BackendId::ULTRAHONK.to_owned(),
                        circuit,
                        version,
                    }
                }
                _ => ProviderError::ArtifactIntegrity { circuit, version },
            }
        })?;
        let bytecode_name = format!("{}.json", request.circuit);
        let bytecode_bytes =
            loaded
                .file(&bytecode_name)
                .ok_or_else(|| ProviderError::ArtifactUnavailable {
                    backend: BackendId::ULTRAHONK.to_owned(),
                    circuit: request.circuit.clone(),
                    version: request.circuit_version,
                })?;
        // bb consumes the file at its verified path; the loader has already
        // proven those bytes match the manifest.
        let bytecode = artifact_dir.join(&bytecode_name);
        let artifact_checksum = ArtifactChecksum::from_bytes(bytecode_bytes);
        let vk_id = VerificationKeyIdPolicy::id_for(
            &request.circuit,
            &request.circuit_version,
            &artifact_checksum,
        );
        let vk_id = crucible_interfaces::VerificationKeyId::new(vk_id).map_err(|e| {
            ProviderError::Internal {
                reason: format!("cannot form verification key id: {e}"),
            }
        })?;

        // Solve the witness against the real circuit package in a scratch
        // workspace. The scratch (holding the private Prover.toml) must
        // outlive the bb prove below, so it lives for this whole scope.
        let scratch = tempfile::tempdir().map_err(|e| ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: format!("cannot create scratch dir: {e}"),
        })?;
        let witness = solve_witness(&self.config, &nargo, request, scratch.path())?;

        // Prove with bb, writing the verification key alongside.
        let out_dir = tempfile::tempdir().map_err(|e| ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: format!("cannot create proof scratch dir: {e}"),
        })?;
        let artifacts = prove(
            &bb,
            &ProveOptions {
                bytecode: &bytecode,
                witness: &witness,
                output_dir: out_dir.path(),
                write_vk: true,
            },
        )
        .map_err(|e| ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: e.to_string(),
        })?;

        // Name the public input words using the pinned circuit surface. A
        // count mismatch means the compiled artifact and the expectations
        // spec have drifted — fail loudly instead of naming words wrongly.
        let spec = expectations(request.operation);
        let words = &artifacts.public_inputs.public_inputs;
        if words.len() != spec.public_word_count() {
            return Err(ProviderError::ProofGeneration {
                backend: BackendId::ULTRAHONK.to_owned(),
                reason: format!(
                    "circuit `{}` reported {} public input words but its pinned surface has {} \
                     (circuit source and expectations spec drifted)",
                    request.circuit,
                    words.len(),
                    spec.public_word_count()
                ),
            });
        }
        let mut public_outputs = OutputBag::new();
        for (name, word) in spec.public_names().zip(words.iter()) {
            let value = field_from_word(word).map_err(|reason| ProviderError::ProofGeneration {
                backend: BackendId::ULTRAHONK.to_owned(),
                reason,
            })?;
            public_outputs
                .insert((*name).to_owned(), value)
                .map_err(|e| ProviderError::Internal {
                    reason: format!("cannot assemble public outputs: {e}"),
                })?;
        }

        // Persist the verification key under its id so verifiers can resolve
        // it without the proof ever carrying key material.
        if let Some(vk) = &artifacts.vk {
            self.config
                .vk_store
                .put(&vk_id, vk)
                .map_err(|e| ProviderError::ProofGeneration {
                    backend: BackendId::ULTRAHONK.to_owned(),
                    reason: format!("cannot store verification key: {e}"),
                })?;
        }

        let proof_bytes =
            artifacts
                .proof
                .proof_bytes()
                .map_err(|e| ProviderError::ProofGeneration {
                    backend: BackendId::ULTRAHONK.to_owned(),
                    reason: format!("cannot decode proof bytes: {e}"),
                })?;

        Ok(ProofResponse::new(
            request.request_id.clone(),
            request.circuit.clone(),
            request.circuit_version,
            self.backend(),
            ProofBlob::new(
                ProofFormat::new(PROOF_FORMAT_TAG).expect("ultrahonk format tag is valid"),
                proof_bytes,
            ),
            public_outputs,
            vk_id,
            artifact_checksum,
            request.state_reference.clone(),
        ))
    }
}

/// Solves `request`'s witness against the real circuit package in a scratch
/// workspace rooted at `scratch_root`, returning the path of the solved
/// witness. The caller keeps `scratch_root` alive until proving completes.
fn solve_witness(
    config: &UltraHonkConfig,
    nargo: &NoirToolchain,
    request: &ProofRequest,
    scratch_root: &std::path::Path,
) -> Result<PathBuf, ProviderError> {
    let operation_name = request.circuit.as_str();
    copy_package_sources(&config.circuits_root, operation_name, scratch_root).map_err(|e| {
        ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: e,
        }
    })?;

    let package_dir = scratch_root.join(operation_name);

    // Drop the precompiled bytecode into the scratch package so nargo
    // executes the witness without recompiling.
    let bytecode = config
        .circuits_root
        .join("target")
        .join(format!("{operation_name}.json"));
    let target_dir = package_dir.join("target");
    std::fs::create_dir_all(&target_dir).map_err(|e| ProviderError::ProofGeneration {
        backend: BackendId::ULTRAHONK.to_owned(),
        reason: format!("cannot create scratch target dir: {e}"),
    })?;
    std::fs::copy(&bytecode, target_dir.join(format!("{operation_name}.json"))).map_err(|e| {
        ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: format!("cannot copy bytecode into scratch: {e}"),
        }
    })?;

    // Prover.toml is the single place private values leave memory; the
    // witness encoder writes it 0600. Values are hex-prefixed so nargo never
    // misreads them as decimal (see the witness crate encoder).
    let data = crucible_witness::WitnessData::from_request(request);
    crucible_witness::encoder::write_prover_toml(&data, &package_dir.join("Prover.toml")).map_err(
        |e| ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: format!("cannot write witness Prover.toml: {e}"),
        },
    )?;

    let captured = nargo
        .execute_captured(&package_dir, "Prover", "witness")
        .map_err(|e| ProviderError::BackendUnavailable {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: format!("nargo execute failed: {e}"),
        })?;
    if !captured.solved {
        return Err(ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: "the witness did not solve: the request's private values do not satisfy the \
                     circuit constraints (wrong owner, overdraw, or opening mismatch)"
                .to_owned(),
        });
    }
    captured
        .witness_path
        .ok_or_else(|| ProviderError::ProofGeneration {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: "nargo reported a solved witness but wrote no witness file".to_owned(),
        })
}

/// Parses one backend field word into a canonical [`FieldValue`].
fn field_from_word(word: &str) -> Result<FieldValue, String> {
    let bare = word.strip_prefix("0x").unwrap_or(word);
    let trimmed = bare.trim_start_matches('0');
    let canonical = if trimmed.is_empty() { "0" } else { trimmed };
    FieldValue::from_hex(canonical)
        .map_err(|e| format!("public input word `{word}` is not canonical: {e}"))
}

/// Copies a circuit package (sources only) plus its `lib/` dependency into a
/// scratch workspace at `scratch_root`, preserving the `../lib` relative
/// path nargo resolves.
fn copy_package_sources(
    circuits_root: &std::path::Path,
    op: &str,
    scratch_root: &std::path::Path,
) -> Result<(), String> {
    let package = circuits_root.join(op);
    for required in ["Nargo.toml", "src"] {
        if !package.join(required).exists() {
            return Err(format!(
                "circuit package `{op}` is missing `{required}` under `{}`",
                circuits_root.display()
            ));
        }
    }
    copy_tree(
        &package.join("Nargo.toml"),
        &scratch_root.join(op).join("Nargo.toml"),
    )?;
    copy_tree(&package.join("src"), &scratch_root.join(op).join("src"))?;
    let lib = circuits_root.join("lib");
    copy_tree(
        &lib.join("Nargo.toml"),
        &scratch_root.join("lib").join("Nargo.toml"),
    )?;
    copy_tree(&lib.join("src"), &scratch_root.join("lib").join("src"))?;
    Ok(())
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_words_parse_to_canonical_values() {
        assert_eq!(
            field_from_word("0x06871944eb38ea75866d42609302692a55e12cf7620a50f2cf03381b9b382b72")
                .unwrap()
                .as_hex(),
            "6871944eb38ea75866d42609302692a55e12cf7620a50f2cf03381b9b382b72"
        );
        assert_eq!(field_from_word("0x00").unwrap().as_hex(), "0");
        assert_eq!(
            field_from_word("0x746f6b656e").unwrap().as_hex(),
            "746f6b656e"
        );
    }

    #[test]
    fn malformed_field_words_are_rejected() {
        assert!(field_from_word("not-hex").is_err());
        let wide = format!("0x{}", "ab".repeat(65));
        assert!(field_from_word(&wide).is_err());
    }
}
