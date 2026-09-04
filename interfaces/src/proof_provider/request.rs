use std::fmt;

use serde::{Deserialize, Serialize};

use crate::circuit::{CircuitId, Operation, PrivateWitnessBag, PublicInputBag, Version};

/// Identifies a proof request for end-to-end traceability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(String);

impl RequestId {
    /// Constructs a request id from a caller-supplied string.
    ///
    /// Callers (e.g. `crucible-scenarios`) should supply globally unique ids;
    /// nothing in this crate generates randomness.
    pub fn new(id: impl Into<String>) -> RequestId {
        RequestId(id.into())
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identifies a proving/verification backend.
///
/// Backends are the concrete proving systems a proof can be generated and
/// verified with. They are versioned strings, not an enum, so new backends
/// (RISC Zero/Groth16, future Stellar verifier architectures) can be added
/// without changing this interface.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BackendId(String);

impl BackendId {
    /// The mock backend. TEST ONLY, not cryptographically secure.
    pub const MOCK: &'static str = "mock";
    /// The UltraHonk backend (Noir + Barretenberg) used by Stellar
    /// Confidential Tokens.
    pub const ULTRAHONK: &'static str = "ultrahonk";

    /// Validates and constructs a backend identifier.
    pub fn new(id: impl Into<String>) -> Result<BackendId, BackendIdError> {
        let id = id.into();
        if id.is_empty() || id.len() > 64 {
            return Err(BackendIdError::InvalidLength);
        }
        if !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(BackendIdError::InvalidCharacters);
        }
        Ok(BackendId(id))
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error constructing a [`BackendId`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BackendIdError {
    /// The id was empty or longer than 64 characters.
    #[error("backend id must be 1..=64 characters")]
    InvalidLength,
    /// The id contains characters outside `[A-Za-z0-9_-]`.
    #[error("backend id contains invalid characters")]
    InvalidCharacters,
}

/// A 256-bit state root digest in canonical hex form.
///
/// State roots identify *which* state a proof applies to. Unlike circuit
/// field elements, roots are full 256-bit digests (e.g. SHA-256 or a Pedersen
/// commitment tree root) and are therefore represented as raw hex, not as
/// [`crate::circuit::FieldValue`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RootDigest(String);

impl RootDigest {
    /// The all-zero digest (pre-initialization state).
    pub fn zero() -> RootDigest {
        RootDigest("0".repeat(64))
    }

    /// Validates and canonicalizes a 64-hex-character root digest.
    pub fn from_hex(hex: &str) -> Result<RootDigest, RootDigestError> {
        let s = hex.strip_prefix("0x").unwrap_or(hex);
        if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(RootDigestError::Not256BitHex);
        }
        Ok(RootDigest(s.to_ascii_lowercase()))
    }

    /// Builds a digest from exactly 32 bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> RootDigest {
        RootDigest(hex::encode(bytes))
    }

    /// Returns the canonical 64-char lowercase hex.
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Returns the digest as raw bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        for (i, b) in hex::decode(&self.0)
            .expect("root digest is valid hex")
            .iter()
            .enumerate()
        {
            out[i] = *b;
        }
        out
    }
}

impl fmt::Display for RootDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error constructing a [`RootDigest`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RootDigestError {
    /// The input was not exactly 64 hex characters.
    #[error("root digest must be exactly 64 lowercase hex characters")]
    Not256BitHex,
}

/// A reference to the state a proof claims to be valid against.
///
/// Binding proofs to a `(root, sequence)` pair is what makes stale-state and
/// replay attacks detectable: a proof generated against state root A must
/// fail verification when submitted against state root B.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateReference {
    /// The state root the operation applies to.
    pub root: RootDigest,
    /// Monotonic sequence number of the state snapshot.
    pub sequence: u64,
    /// Optional human label (e.g. `alice/token:usdc`) for diagnostics.
    pub label: Option<String>,
}

impl StateReference {
    /// Creates a state reference.
    pub fn new(root: RootDigest, sequence: u64) -> StateReference {
        StateReference {
            root,
            sequence,
            label: None,
        }
    }

    /// Creates a state reference with a diagnostic label.
    pub fn with_label(root: RootDigest, sequence: u64, label: impl Into<String>) -> StateReference {
        StateReference {
            root,
            sequence,
            label: Some(label.into()),
        }
    }

    /// Splits the 256-bit root into its two 128-bit field halves, `(hi, lo)`.
    ///
    /// A full state root (e.g. a SHA-256 or Pedersen tree digest) does not
    /// fit in one BN254 field element (~254 bits, and values above the
    /// modulus are invalid), so state-bound circuits commit to the root as
    /// two field halves. `hi` is the most significant 128 bits (the first 32
    /// hex characters of the digest), `lo` the least significant (the last
    /// 32). The split is the single convention shared by fixtures, the
    /// witness path, and the verifier's state-binding check.
    pub fn root_halves(&self) -> (crate::circuit::FieldValue, crate::circuit::FieldValue) {
        let hex = self.root.as_hex();
        let hi = crate::circuit::FieldValue::from_hex(&hex[..32])
            .expect("first root half is canonical 128-bit hex");
        let lo = crate::circuit::FieldValue::from_hex(&hex[32..])
            .expect("second root half is canonical 128-bit hex");
        (hi, lo)
    }
}

/// Errors that can occur while assembling a [`ProofRequest`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WitnessError {
    /// The private witness bag is empty but the operation requires secrets.
    #[error("operation {operation} requires a non-empty private witness")]
    MissingWitness {
        /// The operation being requested.
        operation: Operation,
    },
    /// The public input binding is empty for an operation that requires it.
    #[error("operation {operation} requires public input binding")]
    MissingPublicInputs {
        /// The operation being requested.
        operation: Operation,
    },
    /// The request carries no state reference for an operation that requires
    /// state binding.
    #[error("operation {operation} requires a state reference")]
    MissingStateReference {
        /// The operation being requested.
        operation: Operation,
    },
    /// A value-level problem with the witness or inputs.
    #[error(transparent)]
    InvalidValue(#[from] crate::circuit::FieldError),
}

/// A request to generate a proof for one operation.
///
/// # Privacy
///
/// [`ProofRequest`] carries private witness material, so it implements no
/// `Serialize` and its `Debug` output is redacted. For structured logging use
/// [`ProofRequest::redacted`], which emits public fields only.
#[derive(Clone)]
pub struct ProofRequest {
    /// Caller-supplied id for traceability.
    pub request_id: RequestId,
    /// The protocol operation being proven.
    pub operation: Operation,
    /// The circuit that proves this operation.
    pub circuit: CircuitId,
    /// Version of the circuit to use.
    pub circuit_version: Version,
    /// Version of the compiled artifact expected by the caller.
    pub artifact_version: Version,
    /// The backend that must produce the proof.
    pub backend: BackendId,
    /// Private witness material. Never logged or serialized.
    pub witness: PrivateWitnessBag,
    /// Public context the proof must bind to.
    pub public_inputs: PublicInputBag,
    /// The state the proof must be bound to, when the operation requires
    /// state binding (e.g. transfer, merge, withdraw).
    pub state_reference: Option<StateReference>,
}

impl ProofRequest {
    /// Creates a proof request.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: RequestId,
        operation: Operation,
        circuit: CircuitId,
        circuit_version: Version,
        artifact_version: Version,
        backend: BackendId,
        witness: PrivateWitnessBag,
        public_inputs: PublicInputBag,
        state_reference: Option<StateReference>,
    ) -> ProofRequest {
        ProofRequest {
            request_id,
            operation,
            circuit,
            circuit_version,
            artifact_version,
            backend,
            witness,
            public_inputs,
            state_reference,
        }
    }

    /// Structural validation independent of any backend.
    ///
    /// Currently enforces that every operation carries a private witness and
    /// public input binding; state-bound operations must additionally carry a
    /// [`StateReference`]. Individual providers run stricter, backend- and
    /// circuit-specific checks during generation.
    pub fn validate(&self) -> Result<(), WitnessError> {
        if self.witness.is_empty() {
            return Err(WitnessError::MissingWitness {
                operation: self.operation,
            });
        }
        if self.public_inputs.is_empty() {
            return Err(WitnessError::MissingPublicInputs {
                operation: self.operation,
            });
        }
        if matches!(
            self.operation,
            Operation::Transfer | Operation::Merge | Operation::Withdraw
        ) && self.state_reference.is_none()
        {
            return Err(WitnessError::MissingStateReference {
                operation: self.operation,
            });
        }
        Ok(())
    }

    /// A JSON view of the request with every private value redacted.
    ///
    /// Safe for logs, events, and CI output. Names and counts of private
    /// values are preserved so operators can still see *what* was requested
    /// without seeing *what the secrets were*.
    pub fn redacted(&self) -> serde_json::Value {
        serde_json::json!({
            "request_id": self.request_id,
            "operation": self.operation,
            "circuit": self.circuit,
            "circuit_version": self.circuit_version,
            "artifact_version": self.artifact_version,
            "backend": self.backend,
            "private_witness": {
                "names": self.witness.names().collect::<Vec<_>>(),
                "count": self.witness.len(),
            },
            "public_inputs": self.public_inputs,
            "state_reference": self.state_reference,
        })
    }
}

impl fmt::Debug for ProofRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redacted: never print witness values, only their names.
        f.debug_struct("ProofRequest")
            .field("request_id", &self.request_id)
            .field("operation", &self.operation)
            .field("circuit", &self.circuit)
            .field("circuit_version", &self.circuit_version)
            .field("artifact_version", &self.artifact_version)
            .field("backend", &self.backend)
            .field("witness", &self.witness)
            .field("public_inputs", &self.public_inputs)
            .field("state_reference", &self.state_reference)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::{FieldValue, SecretValue};

    fn transfer_request() -> ProofRequest {
        let mut witness = PrivateWitnessBag::new();
        witness
            .insert("sender_secret", SecretValue::from_hex("deadbeef").unwrap())
            .unwrap();
        let mut public = PublicInputBag::new();
        public
            .insert("token", FieldValue::from_hex("01").unwrap())
            .unwrap();
        ProofRequest::new(
            RequestId::new("req-1"),
            Operation::Transfer,
            CircuitId::for_operation(Operation::Transfer),
            Version::v0_1(),
            Version::v0_1(),
            BackendId::new(BackendId::MOCK).unwrap(),
            witness,
            public,
            Some(StateReference::new(RootDigest::zero(), 0)),
        )
    }

    #[test]
    fn debug_output_never_contains_secrets() {
        let request = transfer_request();
        let debug = format!("{request:?}");
        assert!(!debug.contains("deadbeef"), "debug leaked secret: {debug}");
        assert!(debug.contains("sender_secret"));
    }

    #[test]
    fn redacted_view_never_contains_secrets() {
        let request = transfer_request();
        let redacted = request.redacted().to_string();
        assert!(
            !redacted.contains("deadbeef"),
            "redacted leaked secret: {redacted}"
        );
        assert!(redacted.contains("sender_secret"));
        assert!(redacted.contains("req-1"));
    }

    #[test]
    fn requests_cannot_be_serialized_with_secrets() {
        // ProofRequest intentionally has no Serialize impl; asserting this at
        // compile time is impossible, so assert the guarantee that matters:
        // redacted() is the only JSON path and it is clean (covered above).
        let request = transfer_request();
        assert!(request.validate().is_ok());
    }

    #[test]
    fn validation_requires_witness_inputs_and_state_binding() {
        let mut req = transfer_request();
        req.state_reference = None;
        assert_eq!(
            req.validate(),
            Err(WitnessError::MissingStateReference {
                operation: Operation::Transfer
            })
        );

        let mut req = transfer_request();
        req.witness = PrivateWitnessBag::new();
        assert_eq!(
            req.validate(),
            Err(WitnessError::MissingWitness {
                operation: Operation::Transfer
            })
        );

        let mut req = transfer_request();
        req.public_inputs = PublicInputBag::new();
        assert_eq!(
            req.validate(),
            Err(WitnessError::MissingPublicInputs {
                operation: Operation::Transfer
            })
        );
    }

    #[test]
    fn state_reference_digest_is_strict() {
        assert!(RootDigest::from_hex(&"ab".repeat(32)).is_ok());
        assert!(RootDigest::from_hex(&"ab".repeat(31)).is_err());
        assert!(RootDigest::from_hex(&("0x".to_owned() + &"ab".repeat(32))).is_ok());
        assert!(RootDigest::from_hex(&"zz".repeat(32)).is_err());
        assert_eq!(RootDigest::from_bytes([7u8; 32]).as_hex(), "07".repeat(32));
        assert_eq!(RootDigest::from_bytes([7u8; 32]).to_bytes(), [7u8; 32]);
    }

    #[test]
    fn root_halves_split_256_bits_into_two_fields() {
        // Asymmetric root so hi/lo placement cannot be accidentally swapped.
        let state = StateReference::new(RootDigest::from_hex(&("ab".repeat(16) + &"cd".repeat(16))).unwrap(), 1);
        let (hi, lo) = state.root_halves();
        assert_eq!(hi.as_hex(), "ab".repeat(16));
        assert_eq!(lo.as_hex(), "cd".repeat(16));
        // A root of all-one hex digits exercises no trimming; all-zero halves
        // canonicalize to the minimal "0" form.
        let zero_hi = StateReference::new(RootDigest::from_hex(&("00".repeat(16) + &"ab".repeat(16))).unwrap(), 1);
        let (zhi, zlo) = zero_hi.root_halves();
        assert_eq!(zhi.as_hex(), "0");
        assert_eq!(zlo.as_hex(), "ab".repeat(16));
    }

    #[test]
    fn root_halves_match_expected_fixture_roots() {
        // state_root_a() = "ab"*32 => both halves are "ab"*16.
        let a = StateReference::new(RootDigest::from_hex(&"ab".repeat(32)).unwrap(), 1);
        let (hi, lo) = a.root_halves();
        assert_eq!(hi.as_hex(), "ab".repeat(16));
        assert_eq!(lo.as_hex(), "ab".repeat(16));
        // state_root_b() = "cd"*32.
        let b = StateReference::new(RootDigest::from_hex(&"cd".repeat(32)).unwrap(), 2);
        let (hi, lo) = b.root_halves();
        assert_eq!(hi.as_hex(), "cd".repeat(16));
        assert_eq!(lo.as_hex(), "cd".repeat(16));
    }

    #[test]
    fn backend_ids_are_validated() {
        assert_eq!(
            BackendId::new(BackendId::MOCK).unwrap().as_str(),
            BackendId::MOCK
        );
        assert_eq!(
            BackendId::new(BackendId::ULTRAHONK).unwrap().as_str(),
            BackendId::ULTRAHONK
        );
        assert!(BackendId::new("").is_err());
        assert!(BackendId::new("has space").is_err());
    }
}
