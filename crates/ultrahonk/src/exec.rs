//! Executing `bb prove` and `bb verify`, and parsing the JSON artifacts.
//!
//! All process execution against Barretenberg lives here (mirroring how all
//! `nargo` execution lives behind `crucible-noir`), so callers never build
//! `bb` command lines themselves.
//!
//! # Output format
//!
//! The adapter drives `bb` with `--output_format json`, which makes every
//! run self-describing: each artifact file carries the proving scheme
//! (`ultra_honk`), the exact `bb_version` that produced it, and the
//! backend-native verification-key digest (`vk_hash`). Those three facts are
//! what make a proof reproducible — the same bytes re-verified against a
//! different backend version or a different verification key must fail, and
//! the metadata lets the adapter say *why* up front.
//!
//! # Outcomes vs. errors
//!
//! A proof that fails verification is an **outcome** ([`VerifyOutcome`]),
//! not an error — exactly like an unsatisfiable witness in `crucible-noir`.
//! Errors are reserved for toolchain and I/O failures (missing binaries,
//! missing files, malformed artifacts).
//!
//! # Privacy
//!
//! Witness and bytecode are referenced by path only, never read into this
//! module. Diagnostics carried on errors or outcomes are truncated excerpts
//! of bb's own stderr and never contain witness values.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::errors::UltraHonkError;
use crate::toolchain::BbToolchain;

/// The proving scheme bb reports for Noir ACIR circuits.
pub const SCHEME_ULTRA_HONK: &str = "ultra_honk";

/// `proof.json` — the UltraHonk proof plus backend provenance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProofDocument {
    /// The proof as backend field words (32-byte `0x`-prefixed hex). Public
    /// inputs are embedded in this list per the backend's own layout.
    #[serde(rename = "proof")]
    pub proof: Vec<String>,
    /// Backend-native verification-key digest (`0x` + 64 hex chars).
    #[serde(rename = "vk_hash")]
    pub vk_hash: String,
    /// The bb version that produced this proof.
    #[serde(rename = "bb_version")]
    pub bb_version: String,
    /// The proving scheme (`ultra_honk`).
    pub scheme: String,
}

impl ProofDocument {
    /// The proof as raw bytes (each field word as 32-byte big-endian).
    pub fn proof_bytes(&self) -> Result<Vec<u8>, UltraHonkError> {
        concat_words(&self.proof)
    }

    /// The verification-key digest as raw 32 bytes.
    pub fn vk_hash_bytes(&self) -> Result<[u8; 32], UltraHonkError> {
        digest_bytes(&self.vk_hash)
    }

    /// The verification-key digest in canonical 64-char lowercase hex.
    pub fn vk_hash_hex(&self) -> Result<String, UltraHonkError> {
        Ok(hex::encode(self.vk_hash_bytes()?))
    }
}

/// `public_inputs.json` — the public inputs the proof commits to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PublicInputsDocument {
    /// Public inputs as 32-byte field words, in circuit return order.
    #[serde(rename = "public_inputs")]
    pub public_inputs: Vec<String>,
    /// The bb version that produced this file.
    #[serde(rename = "bb_version")]
    pub bb_version: String,
    /// The proving scheme.
    pub scheme: String,
}

impl PublicInputsDocument {
    /// The public inputs as raw 32-byte big-endian field elements.
    pub fn inputs_bytes(&self) -> Result<Vec<u8>, UltraHonkError> {
        concat_words(&self.public_inputs)
    }
}

/// `vk.json` — the verification key plus its own digest.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VkDocument {
    /// The verification key as backend field words.
    #[serde(rename = "vk")]
    pub vk: Vec<String>,
    /// Backend-native verification-key digest (`0x` + 64 hex chars).
    pub hash: String,
    /// The bb version that produced this file.
    #[serde(rename = "bb_version")]
    pub bb_version: String,
    /// The proving scheme.
    pub scheme: String,
}

impl VkDocument {
    /// The verification key as raw bytes.
    pub fn vk_bytes(&self) -> Result<Vec<u8>, UltraHonkError> {
        concat_words(&self.vk)
    }

    /// The verification-key digest as raw 32 bytes.
    pub fn hash_bytes(&self) -> Result<[u8; 32], UltraHonkError> {
        digest_bytes(&self.hash)
    }
}

/// Options for a `bb prove` run.
#[derive(Debug, Clone)]
pub struct ProveOptions<'a> {
    /// Path to the compiled ACIR bytecode JSON (`nargo compile` output).
    pub bytecode: &'a Path,
    /// Path to the solved witness (`.gz` from `nargo execute`).
    pub witness: &'a Path,
    /// Directory bb writes its artifacts into.
    pub output_dir: &'a Path,
    /// Whether to also write the verification key (`--write_vk`).
    pub write_vk: bool,
}

/// The artifacts a successful `bb prove` produced.
#[derive(Debug, Clone)]
pub struct ProvenArtifacts {
    /// Directory the artifacts were written into.
    pub output_dir: PathBuf,
    /// Parsed `proof.json`.
    pub proof: ProofDocument,
    /// Parsed `public_inputs.json`.
    pub public_inputs: PublicInputsDocument,
    /// Parsed `vk.json`, when [`ProveOptions::write_vk`] was set.
    pub vk: Option<VkDocument>,
}

impl ProvenArtifacts {
    /// Paths of the written artifact files, in the shape `bb verify`
    /// expects them.
    pub fn files(&self) -> ArtifactFiles {
        ArtifactFiles {
            proof: self.output_dir.join("proof.json"),
            vk: self
                .vk
                .as_ref()
                .map(|_| self.output_dir.join("vk.json")),
            public_inputs: self.output_dir.join("public_inputs.json"),
        }
    }
}

/// File paths of a produced proof bundle, for downstream `bb verify` runs.
#[derive(Debug, Clone)]
pub struct ArtifactFiles {
    /// The proof file (`proof.json`).
    pub proof: PathBuf,
    /// The verification key file (`vk.json`), when it was written.
    pub vk: Option<PathBuf>,
    /// The public inputs file (`public_inputs.json`).
    pub public_inputs: PathBuf,
}

/// Options for a `bb verify` run.
#[derive(Debug, Clone)]
pub struct VerifyOptions<'a> {
    /// Path to the proof (`proof.json`).
    pub proof: &'a Path,
    /// Path to the verification key (`vk.json`).
    pub vk: &'a Path,
    /// Path to the public inputs (`public_inputs.json`).
    pub public_inputs: &'a Path,
}

/// The verdict of a `bb verify` run.
///
/// A rejected proof is a first-class outcome, not an error: the request was
/// serviced, the backend said no.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyOutcome {
    /// Whether the proof verified.
    pub verified: bool,
    /// Short redacted backend diagnostic, when the run failed.
    pub diagnostic: Option<String>,
}

/// Runs `bb prove` against a solved witness and compiled bytecode.
///
/// The witness and bytecode are referenced by path and never read here. On
/// success the produced JSON artifacts are parsed, cross-checked (the VK's
/// own digest must equal the digest the proof embeds), and returned.
pub fn prove(
    toolchain: &BbToolchain,
    options: &ProveOptions,
) -> Result<ProvenArtifacts, UltraHonkError> {
    toolchain.check_version()?;
    require_file(options.bytecode)?;
    require_file(options.witness)?;
    std::fs::create_dir_all(options.output_dir).map_err(|e| UltraHonkError::Io {
        path: options.output_dir.display().to_string(),
        reason: e.to_string(),
    })?;

    let mut command = Command::new(toolchain.binary());
    command.arg("prove");
    command.arg("-b").arg(options.bytecode);
    command.arg("-w").arg(options.witness);
    command.arg("-o").arg(options.output_dir);
    if options.write_vk {
        command.arg("--write_vk");
    }
    command.arg("--output_format").arg("json");

    let output = command
        .output()
        .map_err(|e| UltraHonkError::Spawn {
            reason: format!("cannot spawn `{}`: {e}", toolchain.binary().display()),
        })?;
    if !output.status.success() {
        return Err(UltraHonkError::CommandFailed {
            command: "prove".to_owned(),
            reason: excerpt(&String::from_utf8_lossy(&output.stderr)),
        });
    }

    let proof = read_document::<ProofDocument>(&options.output_dir.join("proof.json"))?;
    let public_inputs =
        read_document::<PublicInputsDocument>(&options.output_dir.join("public_inputs.json"))?;
    let vk = if options.write_vk {
        Some(read_document::<VkDocument>(&options.output_dir.join("vk.json"))?)
    } else {
        None
    };

    validate_provenance("proof", &proof.scheme, &proof.bb_version)?;
    validate_provenance("public inputs", &public_inputs.scheme, &public_inputs.bb_version)?;
    if let Some(vk) = &vk {
        validate_provenance("verification key", &vk.scheme, &vk.bb_version)?;
        // The proof embeds the digest of the exact VK it must be checked
        // against; a mismatch means bb wrote inconsistent artifacts.
        let proof_hash = proof.vk_hash_bytes()?;
        if vk.hash_bytes()? != proof_hash {
            return Err(UltraHonkError::InconsistentArtifacts {
                reason: "proof embeds a different verification-key digest than vk.json".to_owned(),
            });
        }
    }

    Ok(ProvenArtifacts {
        output_dir: options.output_dir.to_path_buf(),
        proof,
        public_inputs,
        vk,
    })
}

/// Runs `bb verify` against a produced proof bundle.
///
/// Exit 0 means the proof verified; any other exit is reported as a rejected
/// [`VerifyOutcome`] with a short diagnostic. Toolchain and I/O problems are
/// errors.
pub fn verify(
    toolchain: &BbToolchain,
    options: &VerifyOptions,
) -> Result<VerifyOutcome, UltraHonkError> {
    toolchain.check_version()?;
    require_file(options.proof)?;
    require_file(options.vk)?;
    require_file(options.public_inputs)?;

    let output = Command::new(toolchain.binary())
        .arg("verify")
        .arg("-p")
        .arg(options.proof)
        .arg("-k")
        .arg(options.vk)
        .arg("-i")
        .arg(options.public_inputs)
        .output()
        .map_err(|e| UltraHonkError::Spawn {
            reason: format!("cannot spawn `{}`: {e}", toolchain.binary().display()),
        })?;

    if output.status.success() {
        return Ok(VerifyOutcome {
            verified: true,
            diagnostic: None,
        });
    }
    Ok(VerifyOutcome {
        verified: false,
        diagnostic: Some(excerpt(&String::from_utf8_lossy(&output.stderr))),
    })
}

/// Reads and parses one bb JSON artifact.
fn read_document<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, UltraHonkError> {
    let text = std::fs::read_to_string(path).map_err(|e| UltraHonkError::Io {
        path: path.display().to_string(),
        reason: e.to_string(),
    })?;
    serde_json::from_str(&text).map_err(|e| UltraHonkError::MalformedArtifact {
        path: path.display().to_string(),
        reason: e.to_string(),
    })
}

/// Validates that an artifact was produced by the scheme/version this
/// adapter understands.
fn validate_provenance(what: &str, scheme: &str, bb_version: &str) -> Result<(), UltraHonkError> {
    if scheme != SCHEME_ULTRA_HONK {
        return Err(UltraHonkError::InconsistentArtifacts {
            reason: format!("{what} declares scheme `{scheme}`, expected `{SCHEME_ULTRA_HONK}`"),
        });
    }
    if bb_version.trim().is_empty() {
        return Err(UltraHonkError::MalformedArtifact {
            path: "<document>".to_owned(),
            reason: format!("{what} carries no bb_version"),
        });
    }
    Ok(())
}

/// Concatenates `0x`-prefixed field words into raw 32-byte big-endian bytes.
fn concat_words(words: &[String]) -> Result<Vec<u8>, UltraHonkError> {
    let mut out = Vec::with_capacity(words.len() * 32);
    for word in words {
        out.extend_from_slice(&word_bytes(word)?);
    }
    Ok(out)
}

/// Parses one `0x`-prefixed field word into exactly 32 bytes.
fn word_bytes(word: &str) -> Result<[u8; 32], UltraHonkError> {
    let hex = word.strip_prefix("0x").unwrap_or(word);
    if hex.is_empty() || hex.len() > 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(UltraHonkError::MalformedArtifact {
            path: "<field word>".to_owned(),
            reason: format!("word `{word}` is not 0x-prefixed hex of at most 32 bytes"),
        });
    }
    let padded = format!("{hex:0>64}");
    let mut out = [0u8; 32];
    out.copy_from_slice(
        &hex::decode(&padded).expect("validated hex is decodable")[..],
    );
    Ok(out)
}

/// Parses a `0x`-prefixed digest that must be exactly 32 bytes.
fn digest_bytes(hex: &str) -> Result<[u8; 32], UltraHonkError> {
    let bare = hex.strip_prefix("0x").unwrap_or(hex);
    if bare.len() != 64 || !bare.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(UltraHonkError::MalformedArtifact {
            path: "<digest>".to_owned(),
            reason: format!("digest `{hex}` is not exactly 32 bytes of hex"),
        });
    }
    word_bytes(hex)
}

/// Checks that a required input file exists.
fn require_file(path: &Path) -> Result<(), UltraHonkError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(UltraHonkError::MissingFile {
            path: path.display().to_string(),
        })
    }
}

/// Redacts bb's stderr into a short diagnostic: last non-empty line, with
/// timing noise stripped. Never echoes raw stderr wholesale.
fn excerpt(stderr: &str) -> String {
    let line = stderr
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("bb failed (no diagnostic)")
        .trim();
    let stripped = line
        .split(" (mem:")
        .next()
        .unwrap_or(line)
        .trim_end()
        .to_owned();
    let mut out = stripped;
    if out.len() > 200 {
        out.truncate(200);
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROOF_JSON: &str = r#"{
        "proof": ["0x00", "0x1234", "0xdeadbeef"],
        "vk_hash": "0x12804588d2137c4293a920afbd63c968d8e847a0cf59704e58440ea0fb7d5cf9",
        "bb_version": "6.0.0-nightly.20260903",
        "scheme": "ultra_honk"
    }"#;

    const PUBLIC_INPUTS_JSON: &str = r#"{
        "public_inputs": ["0x06871944eb38ea75866d42609302692a55e12cf7620a50f2cf03381b9b382b72"],
        "bb_version": "6.0.0-nightly.20260903",
        "scheme": "ultra_honk"
    }"#;

    const VK_JSON: &str = r#"{
        "vk": ["0x0e", "0x09", "0x05"],
        "hash": "0x12804588d2137c4293a920afbd63c968d8e847a0cf59704e58440ea0fb7d5cf9",
        "bb_version": "6.0.0-nightly.20260903",
        "scheme": "ultra_honk"
    }"#;

    #[test]
    fn parses_backend_documents() {
        let proof: ProofDocument = serde_json::from_str(PROOF_JSON).unwrap();
        assert_eq!(proof.scheme, SCHEME_ULTRA_HONK);
        assert_eq!(proof.bb_version, "6.0.0-nightly.20260903");
        assert_eq!(proof.proof.len(), 3);
        assert_eq!(proof.vk_hash_bytes().unwrap().len(), 32);
        assert_eq!(
            proof.vk_hash_hex().unwrap(),
            "12804588d2137c4293a920afbd63c968d8e847a0cf59704e58440ea0fb7d5cf9"
        );

        let pi: PublicInputsDocument = serde_json::from_str(PUBLIC_INPUTS_JSON).unwrap();
        assert_eq!(pi.public_inputs.len(), 1);
        assert_eq!(pi.inputs_bytes().unwrap().len(), 32);

        let vk: VkDocument = serde_json::from_str(VK_JSON).unwrap();
        assert_eq!(vk.hash_bytes().unwrap(), proof.vk_hash_bytes().unwrap());
        assert_eq!(vk.vk_bytes().unwrap().len(), 96);
    }

    #[test]
    fn field_words_concatenate_as_big_endian_words() {
        let doc: ProofDocument = serde_json::from_str(PROOF_JSON).unwrap();
        let bytes = doc.proof_bytes().unwrap();
        assert_eq!(bytes.len(), 3 * 32);
        // Last word 0xdeadbeef → last four bytes.
        assert_eq!(&bytes[32 * 3 - 4..], &[0xde, 0xad, 0xbe, 0xef]);
        // Words are zero-padded on the left.
        assert_eq!(&bytes[32..32 + 30], &[0u8; 30]);
    }

    #[test]
    fn overwide_or_malformed_words_are_rejected() {
        assert!(word_bytes("0x00").is_ok());
        assert!(word_bytes("00").is_ok()); // bare hex accepted
        let too_wide = format!("0x{}", "ab".repeat(65));
        assert!(word_bytes(&too_wide).is_err());
        assert!(word_bytes("not-hex").is_err());
        assert!(word_bytes("").is_err());
        assert!(digest_bytes("0x1234").is_err());
        assert!(digest_bytes("zz".repeat(32).as_str()).is_err());
    }

    #[test]
    fn provenance_validation_rejects_foreign_schemes_and_blank_versions() {
        assert!(validate_provenance("proof", SCHEME_ULTRA_HONK, "6.0.0-nightly.20260903").is_ok());
        assert!(validate_provenance("proof", "chonk", "6.0.0").is_err());
        assert!(validate_provenance("proof", SCHEME_ULTRA_HONK, "").is_err());
    }

    #[test]
    fn artifact_digest_mismatch_is_detected() {
        let proof: ProofDocument = serde_json::from_str(PROOF_JSON).unwrap();
        let other_vk = VK_JSON.replace(
            "12804588d2137c4293a920afbd63c968d8e847a0cf59704e58440ea0fb7d5cf9",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        // If the replace did not apply (test drift), the test is meaningless.
        assert_ne!(other_vk, VK_JSON.to_owned());
        let vk: VkDocument = serde_json::from_str(&other_vk).unwrap();
        assert_ne!(vk.hash_bytes().unwrap(), proof.vk_hash_bytes().unwrap());
    }

    #[test]
    fn excerpts_are_short_and_redacted() {
        let raw = "Scheme is: ultra_honk, num threads: 2 (mem: 4.88 MiB)\n\
                   UltraVerifier: verification failed at reduction step (mem: 7.88 MiB)\n\
                   Proof verification failed (mem: 7.88 MiB)\n";
        let out = excerpt(raw);
        assert_eq!(out, "Proof verification failed");
        assert!(!out.contains("(mem:"));
        assert!(!out.contains('\n'));

        // 200 truncated chars plus a multi-byte ellipsis.
        let long = format!("x{}", "y".repeat(500));
        let out = excerpt(&long);
        assert_eq!(out.len(), 203); // bytes: 200 chars + 3-byte '…'
        assert!(out.ends_with('…'));
    }
}
