use std::fmt;

use serde::{Deserialize, Serialize};

use crate::circuit::{CircuitId, OutputBag, Version};
use crate::proof_provider::{BackendId, RequestId, RootDigest, StateReference};

/// The wire format of a proof's bytes.
///
/// The actual cryptographic encoding is owned by the backend that produced
/// the proof; this type only names it so consumers can route proofs to the
/// matching verifier without guessing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProofFormat(String);

impl ProofFormat {
    /// Mock envelope format produced by the test-only mock backend.
    pub const MOCK: &'static str = "mock-envelope-v1";
    /// UltraHonk proof format produced by the Barretenberg backend.
    pub const ULTRAHONK: &'static str = "ultrahonk-v1";

    /// Validates and constructs a proof format tag.
    pub fn new(format: impl Into<String>) -> Result<ProofFormat, ProofFormatError> {
        let format = format.into();
        if format.is_empty() || format.len() > 64 {
            return Err(ProofFormatError::InvalidLength);
        }
        Ok(ProofFormat(format))
    }

    /// Returns the tag as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProofFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error constructing a [`ProofFormat`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProofFormatError {
    /// The tag was empty or longer than 64 characters.
    #[error("proof format tag must be 1..=64 characters")]
    InvalidLength,
}

/// An opaque proof as produced by a backend.
///
/// Proof bytes are backend-specific and never interpreted here. They are
/// transported as hex for serialization stability and verified by a
/// [`crate::verifier::Verifier`] that understands the [`ProofFormat`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofBlob {
    /// Format the bytes are encoded in.
    pub format: ProofFormat,
    /// The raw proof bytes.
    #[serde(with = "hex_bytes")]
    pub bytes: Vec<u8>,
}

impl ProofBlob {
    /// Wraps raw proof bytes with their format.
    pub fn new(format: ProofFormat, bytes: Vec<u8>) -> ProofBlob {
        ProofBlob { format, bytes }
    }
}

/// Serialization for raw bytes as lowercase hex.
mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let raw = String::deserialize(deserializer)?;
        hex::decode(&raw).map_err(serde::de::Error::custom)
    }
}

/// Identifies the verification key a proof must be checked against.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VerificationKeyId(String);

impl VerificationKeyId {
    /// Validates and constructs a verification key id.
    pub fn new(id: impl Into<String>) -> Result<VerificationKeyId, VerificationKeyIdError> {
        let id = id.into();
        if id.is_empty() || id.len() > 128 {
            return Err(VerificationKeyIdError::InvalidLength);
        }
        Ok(VerificationKeyId(id))
    }

    /// Returns the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VerificationKeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error constructing a [`VerificationKeyId`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VerificationKeyIdError {
    /// The id was empty or longer than 128 characters.
    #[error("verification key id must be 1..=128 characters")]
    InvalidLength,
}

/// SHA-256 checksum (64 hex chars) of a compiled artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactChecksum(String);

impl ArtifactChecksum {
    /// Validates and constructs a checksum from canonical hex.
    pub fn from_hex(hex: &str) -> Result<ArtifactChecksum, ArtifactChecksumError> {
        let s = hex.strip_prefix("0x").unwrap_or(hex);
        if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ArtifactChecksumError::NotSha256Hex);
        }
        Ok(ArtifactChecksum(s.to_ascii_lowercase()))
    }

    /// Computes a checksum over arbitrary artifact bytes (SHA-256).
    pub fn from_bytes(bytes: &[u8]) -> ArtifactChecksum {
        use sha2::{Digest, Sha256};
        ArtifactChecksum(hex::encode(Sha256::digest(bytes)))
    }

    /// Returns the canonical 64-char lowercase hex.
    pub fn as_hex(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactChecksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error constructing an [`ArtifactChecksum`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ArtifactChecksumError {
    /// The input was not exactly 64 hex characters.
    #[error("artifact checksum must be exactly 64 lowercase hex characters")]
    NotSha256Hex,
}

/// The result of a successful proof generation.
///
/// A [`ProofResponse`] is the traceable unit every downstream consumer
/// (verifiers, artifacts store, scenario runner) works with: it identifies
/// the circuit, version, backend, verification key, and artifact checksum the
/// proof was produced against, so proofs can be reproduced and audited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofResponse {
    /// Id of the request that produced this proof.
    pub request_id: RequestId,
    /// Circuit the proof was generated for.
    pub circuit: CircuitId,
    /// Circuit version the proof is valid for.
    pub circuit_version: Version,
    /// Backend that generated the proof.
    pub backend: BackendId,
    /// The proof itself.
    pub proof: ProofBlob,
    /// Public outputs the proof commits to.
    pub public_outputs: OutputBag,
    /// Verification key the proof must be checked against.
    pub verification_key_id: VerificationKeyId,
    /// SHA-256 checksum of the artifact that produced the proof.
    pub artifact_checksum: ArtifactChecksum,
    /// State the proof is bound to (mirrors the request when bound).
    pub state_reference: Option<StateReference>,
}

impl ProofResponse {
    /// Creates a proof response.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: RequestId,
        circuit: CircuitId,
        circuit_version: Version,
        backend: BackendId,
        proof: ProofBlob,
        public_outputs: OutputBag,
        verification_key_id: VerificationKeyId,
        artifact_checksum: ArtifactChecksum,
        state_reference: Option<StateReference>,
    ) -> ProofResponse {
        ProofResponse {
            request_id,
            circuit,
            circuit_version,
            backend,
            proof,
            public_outputs,
            verification_key_id,
            artifact_checksum,
            state_reference,
        }
    }

    /// The root digest this proof binds to, when bound.
    pub fn state_root(&self) -> Option<&RootDigest> {
        self.state_reference.as_ref().map(|s| &s.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_blob_serializes_bytes_as_hex() {
        let blob = ProofBlob::new(
            ProofFormat::new(ProofFormat::MOCK).unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef],
        );
        let json = serde_json::to_string(&blob).unwrap();
        assert!(json.contains("\"deadbeef\""), "unexpected json: {json}");
        let back: ProofBlob = serde_json::from_str(&json).unwrap();
        assert_eq!(back, blob);
    }

    #[test]
    fn checksums_are_strict_and_deterministic() {
        assert!(ArtifactChecksum::from_hex(&"ab".repeat(32)).is_ok());
        assert!(ArtifactChecksum::from_hex(&"ab".repeat(31)).is_err());
        let a = ArtifactChecksum::from_bytes(b"circuit bytes");
        let b = ArtifactChecksum::from_bytes(b"circuit bytes");
        let c = ArtifactChecksum::from_bytes(b"circuit bytess");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.as_hex().len(), 64);
    }

    #[test]
    fn proof_format_tags_are_validated() {
        assert!(ProofFormat::new(ProofFormat::ULTRAHONK).is_ok());
        assert!(ProofFormat::new("").is_err());
        assert!(VerificationKeyId::new("vk-transfer-0.1.0").is_ok());
        assert!(VerificationKeyId::new("").is_err());
    }
}
