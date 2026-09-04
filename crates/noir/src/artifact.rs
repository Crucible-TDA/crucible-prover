//! Reading compiled circuit artifacts produced by `nargo compile`.
//!
//! `nargo compile` writes one JSON artifact per package into `target/`
//! containing the ACIR bytecode plus an ABI describing the circuit's
//! parameters and return values. This module parses that file into a typed
//! model and extracts the public-input boundary it declares.
//!
//! # Schema stability
//!
//! The artifact schema is validated against real `nargo` output in
//! integration tests (gated on the toolchain being installed). Unknown JSON
//! fields are ignored, so minor compiler schema additions do not break
//! parsing.

use std::path::Path;

use serde::Deserialize;

use crate::errors::NoirError;

/// The top-level structure of a `nargo compile` artifact (`target/*.json`).
#[derive(Debug, Clone, Deserialize)]
pub struct CompiledArtifact {
    /// Noir compiler version that produced the artifact.
    pub noir_version: String,
    /// Compiler content hash as reported by nargo (format is version
    /// dependent; treated as an opaque string here).
    pub hash: String,
    /// The circuit ABI: parameters and return values.
    pub abi: Abi,
    /// The base64-encoded ACIR bytecode.
    pub bytecode: String,
}

/// The ABI of a compiled circuit.
#[derive(Debug, Clone, Deserialize)]
pub struct Abi {
    /// Named parameters (the circuit's inputs), in declaration order.
    #[serde(default)]
    pub parameters: Vec<AbiParameter>,
    /// The declared return value, when the circuit returns one.
    #[serde(default)]
    pub return_type: Option<ReturnType>,
}

/// One named parameter of a circuit.
#[derive(Debug, Clone, Deserialize)]
pub struct AbiParameter {
    /// Parameter name.
    pub name: String,
    /// Whether the parameter is public (`"public"` or `"private"`).
    #[serde(default)]
    pub visibility: String,
    /// The parameter's type.
    #[serde(rename = "type")]
    pub param_type: AbiType,
}

/// The circuit's declared return value: a type plus its visibility.
#[derive(Debug, Clone, Deserialize)]
pub struct ReturnType {
    /// The return type.
    #[serde(rename = "abi_type")]
    pub abi_type: AbiType,
    /// Visibility of the return value.
    #[serde(default)]
    pub visibility: String,
}

/// A Noir ABI type (a subset sufficient for Crucible's circuits).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AbiType {
    /// A field element (or integer).
    Field,
    /// A boolean.
    Boolean,
    /// A signed or unsigned integer.
    Integer {
        /// Bit width.
        width: u32,
        /// Sign.
        signed: bool,
    },
    /// An array of a fixed length.
    Array {
        /// Element type.
        #[serde(rename = "type")]
        element: Box<AbiType>,
        /// Element count.
        length: usize,
    },
    /// A struct with named fields.
    Struct {
        /// Field list.
        fields: Vec<(String, AbiType)>,
    },
    /// A string of a fixed length.
    String {
        /// Character length.
        length: usize,
    },
}

impl CompiledArtifact {
    /// Parses a compiled artifact from its JSON file.
    pub fn from_file(path: &Path) -> Result<CompiledArtifact, NoirError> {
        let bytes = std::fs::read(path).map_err(|e| NoirError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        Self::from_bytes(&bytes, path)
    }

    /// Parses a compiled artifact from raw JSON bytes.
    pub fn from_bytes(bytes: &[u8], path: &Path) -> Result<CompiledArtifact, NoirError> {
        serde_json::from_slice(bytes).map_err(|e| NoirError::MalformedArtifact {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Public parameter names in declaration order.
    pub fn public_parameter_names(&self) -> Vec<&str> {
        self.abi
            .parameters
            .iter()
            .filter(|p| p.visibility == "public")
            .map(|p| p.name.as_str())
            .collect()
    }

    /// Whether the artifact hash field is non-empty and plausibly
    /// well-formed (either decimal digits or hex, depending on compiler
    /// version).
    pub fn hash_is_well_formed(&self) -> bool {
        !self.hash.is_empty()
            && self.hash.len() <= 64
            && self.hash.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured shape of a real `nargo 1.0.0-beta.26 compile` artifact.
    const SAMPLE: &str = r#"{
      "noir_version": "1.0.0-beta.26",
      "hash": "16175532309056734523",
      "abi": {
        "parameters": [
          { "name": "x", "type": { "kind": "field" }, "visibility": "public" },
          { "name": "y", "type": { "kind": "field" }, "visibility": "private" }
        ],
        "return_type": {
          "abi_type": { "kind": "field" },
          "visibility": "public"
        },
        "error_types": {}
      },
      "bytecode": "H4sIAAAAAAAA/43KPQ5AMBgA0JaLGNmIE4hITGIUiUHCYPCTshh7g36amA0mBxB2F+lmtNj1BHjzUwc",
      "debug_symbols": "...",
      "file_map": {}
    }"#;

    #[test]
    fn parses_artifact_and_extracts_public_boundary() {
        let artifact =
            CompiledArtifact::from_bytes(SAMPLE.as_bytes(), Path::new("t.json")).unwrap();
        assert_eq!(artifact.noir_version, "1.0.0-beta.26");
        assert!(artifact.hash_is_well_formed());
        assert_eq!(artifact.public_parameter_names(), vec!["x"]);
        assert_eq!(artifact.abi.parameters.len(), 2);
        let ret = artifact.abi.return_type.as_ref().unwrap();
        assert_eq!(ret.visibility, "public");
        assert!(matches!(ret.abi_type, AbiType::Field));
    }

    #[test]
    fn malformed_json_is_rejected() {
        let err = CompiledArtifact::from_bytes(b"{not json", Path::new("t.json")).unwrap_err();
        assert!(matches!(err, NoirError::MalformedArtifact { .. }));
    }

    #[test]
    fn hash_validation_is_strict() {
        let artifact =
            CompiledArtifact::from_bytes(SAMPLE.as_bytes(), Path::new("t.json")).unwrap();
        assert!(artifact.hash_is_well_formed());
        let tampered = SAMPLE.replace("16175532309056734523", "zz-not-a-hash");
        let artifact =
            CompiledArtifact::from_bytes(tampered.as_bytes(), Path::new("t.json")).unwrap();
        assert!(!artifact.hash_is_well_formed());
    }
}
