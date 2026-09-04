//! Loading and structurally validating the cross-language test-vector
//! catalog under `test-vectors/`.
//!
//! A test vector is a pure JSON document (see `schemas/test-vector.schema.json`)
//! describing one operation's witness: which values are public, which are
//! private, what public outputs the circuit should report, and whether the
//! vector is expected to verify. Because the format is JSON-only, the same
//! file can drive a Rust runner (this crate), the Python schema checker, and
//! eventually non-Rust consumers.
//!
//! # Privacy boundary
//!
//! Vector witnesses are synthetic test material with **no real secrecy**
//! (sample keys like `0x1234` appear in the circuit sources themselves). The
//! loader nevertheless maps private fields onto [`SecretValue`] at the moment
//! a [`ProofRequest`] is built, so the exact same code path as live proving is
//! exercised. Nothing in this module may ever be used to handle real witness
//! material.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crucible_interfaces::{
    BackendId, CircuitId, FieldValue, Operation, PrivateWitnessBag, ProofRequest, PublicInputBag,
    RequestId, SecretValue, StateReference, Version,
};
use serde::Deserialize;

/// Wire form of one vector file, exactly as JSON on disk (mirrors
/// `test-vector.schema.json`). Private values stay raw strings here so
/// canonicalization happens in one explicit place ([`TestVector::load`]);
/// [`SecretValue`] deliberately has no `Deserialize`.
#[derive(Debug, Clone, Deserialize)]
struct RawVector {
    /// Unique vector id (used for traceability and cross-references).
    id: String,
    /// The operation under test.
    operation: Operation,
    /// Semantic category (drives what the runner asserts).
    category: String,
    /// Optional human description.
    #[serde(default)]
    description: Option<String>,
    /// The circuit id; must equal the operation's canonical circuit.
    circuit: CircuitId,
    /// Circuit version the vector was generated against.
    circuit_version: Version,
    /// Witness inputs (public and private).
    witness: RawWitness,
    /// Public outputs the circuit should report when the witness solves.
    #[serde(rename = "expected_public_outputs")]
    expected_outputs: RawBag,
    /// State binding context, when the operation is state-bound.
    state_reference: Option<StateReference>,
    /// Whether this vector's witness is expected to produce a valid proof.
    expect_verification: bool,
}

/// Raw witness inputs of a vector file.
#[derive(Debug, Clone, Deserialize)]
struct RawWitness {
    /// The operation this witness is for.
    operation: Operation,
    /// Private witness values keyed by name (raw hex).
    private: BTreeMap<String, String>,
    /// Public input values (ordered bag of `[name, hex]` pairs).
    public: RawBag,
}

/// Raw ordered `[name, hex]` bag, matching the schema's `field-bag`.
#[derive(Debug, Clone, Deserialize)]
struct RawBag {
    /// The `[name, value]` pairs in order.
    entries: Vec<(String, String)>,
}

impl RawBag {
    /// Converts into an ordered, canonical [`PublicInputBag`], rejecting
    /// duplicate names and non-canonical hex.
    fn into_bag(self, context: &str) -> Result<PublicInputBag, String> {
        let mut bag = PublicInputBag::new();
        for (name, hex) in self.entries {
            let value = FieldValue::from_hex(&hex)
                .map_err(|e| format!("{context}: field `{name}`: {e}"))?;
            bag.insert(name, value)
                .map_err(|e| format!("{context}: {e}"))?;
        }
        Ok(bag)
    }
}

/// A structurally validated test vector, ready for execution tiers.
#[derive(Debug, Clone)]
pub struct TestVector {
    /// Unique vector id (used for traceability and cross-references).
    pub id: String,
    /// The operation under test.
    pub operation: Operation,
    /// Semantic category (drives what the runner asserts).
    pub category: String,
    /// Optional human description.
    pub description: Option<String>,
    /// The circuit id; must equal the operation's canonical circuit.
    pub circuit: CircuitId,
    /// Circuit version the vector was generated against.
    pub circuit_version: Version,
    /// Witness inputs (public and private).
    pub witness: Witness,
    /// Public outputs the circuit should report when the witness solves.
    pub expected_public_outputs: PublicInputBag,
    /// State binding context, when the operation is state-bound.
    pub state_reference: Option<StateReference>,
    /// Whether this vector's witness is expected to produce a valid proof.
    pub expect_verification: bool,
}

/// Witness inputs of a vector: private values plus ordered public inputs.
#[derive(Debug, Clone)]
pub struct Witness {
    /// The operation this witness is for.
    pub operation: Operation,
    /// Private witness values keyed by name (canonical hex).
    pub private: BTreeMap<String, String>,
    /// Public input values (ordered bag).
    pub public: PublicInputBag,
}

/// Category names that must pair with `expect_verification: false`.
pub const REJECT_CATEGORIES: &[&str] = &[
    "invalid",
    "wrong-owner",
    "insufficient-balance",
    "stale-state",
    "malformed-proof",
    "replay",
];

/// Category names allowed by the JSON schema.
const KNOWN_CATEGORIES: &[&str] = &[
    "valid",
    "invalid",
    "insufficient-balance",
    "wrong-owner",
    "stale-state",
    "malformed-proof",
    "replay",
    "state-transition",
    "commitment",
];

impl TestVector {
    /// Parses and structurally validates a vector file.
    pub fn load(path: &Path) -> Result<TestVector, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
        let raw: RawVector = serde_json::from_str(&text)
            .map_err(|e| format!("`{}` is not a valid test vector: {e}", path.display()))?;
        let context = format!("vector `{}`", raw.id);
        let witness = raw
            .witness
            .into_witness(&context)
            .map_err(|e| format!("{e} (in `{}`)", path.display()))?;
        let expected_public_outputs = raw
            .expected_outputs
            .into_bag(&format!("{context}: expected_public_outputs"))
            .map_err(|e| format!("{e} (in `{}`)", path.display()))?;
        let vector = TestVector {
            id: raw.id,
            operation: raw.operation,
            category: raw.category,
            description: raw.description,
            circuit: raw.circuit,
            circuit_version: raw.circuit_version,
            witness,
            expected_public_outputs,
            state_reference: raw.state_reference,
            expect_verification: raw.expect_verification,
        };
        vector
            .validate_structure()
            .map_err(|e| format!("{e} (in `{}`)", path.display()))?;
        Ok(vector)
    }

    /// Structural validation that must hold for **every** catalog entry,
    /// whether or not its witness is expected to verify.
    pub fn validate_structure(&self) -> Result<(), String> {
        let ctx = |msg: String| format!("vector `{}`: {msg}", self.id);

        if !KNOWN_CATEGORIES.contains(&self.category.as_str()) {
            return Err(ctx(format!("unknown category `{}`", self.category)));
        }
        if self.circuit.as_str() != self.operation.as_str() {
            return Err(ctx(format!(
                "circuit `{}` does not match operation `{}`",
                self.circuit, self.operation
            )));
        }
        if self.witness.operation != self.operation {
            return Err(ctx(format!(
                "witness operation `{}` does not match vector operation `{}`",
                self.witness.operation, self.operation
            )));
        }
        if self.witness.private.is_empty() {
            return Err(ctx(
                "operation requires a non-empty private witness".to_owned()
            ));
        }
        // A private value and a public input must never share a name.
        for name in self.witness.public.names() {
            if self.witness.private.contains_key(name) {
                return Err(ctx(format!(
                    "name `{name}` appears in both private witness and public inputs"
                )));
            }
        }
        // Category/expectation coherence: `valid` must verify; the reject
        // categories must not. The remaining categories
        // (`state-transition`, `commitment`) describe proof-level scenarios
        // that later runners judge explicitly, so no inference is attempted
        // for them here.
        if self.category == "valid" && !self.expect_verification {
            return Err(ctx(
                "category `valid` requires expect_verification=true".to_owned()
            ));
        }
        if REJECT_CATEGORIES.contains(&self.category.as_str()) && self.expect_verification {
            return Err(ctx(format!(
                "category `{}` requires expect_verification=false",
                self.category
            )));
        }
        // State-bound operations carry a state reference whose halves must
        // appear as the circuit's `root_hi`/`root_lo` public params: the
        // witness is only meaningful against the root it names.
        if matches!(
            self.operation,
            Operation::Transfer | Operation::Merge | Operation::Withdraw
        ) {
            let state = self.state_reference.as_ref().ok_or_else(|| {
                ctx(format!(
                    "operation {} requires a state reference",
                    self.operation
                ))
            })?;
            let (hi, lo) = state.root_halves();
            let hi_entry = self.witness.public.get("root_hi").ok_or_else(|| {
                ctx("state-bound operation must expose `root_hi` as a public input".to_owned())
            })?;
            let lo_entry = self.witness.public.get("root_lo").ok_or_else(|| {
                ctx("state-bound operation must expose `root_lo` as a public input".to_owned())
            })?;
            if hi_entry != &hi {
                return Err(ctx(format!(
                    "`root_hi` ({}) does not match the state root's high half ({})",
                    hi_entry.as_hex(),
                    hi.as_hex()
                )));
            }
            if lo_entry != &lo {
                return Err(ctx(format!(
                    "`root_lo` ({}) does not match the state root's low half ({})",
                    lo_entry.as_hex(),
                    lo.as_hex()
                )));
            }
        }
        // register returns nothing, so valid register vectors report no
        // outputs; every other operation reports its public return values.
        if self.operation == Operation::Register && !self.expected_public_outputs.is_empty() {
            return Err(ctx(
                "register has no public return; expected_public_outputs must be empty".to_owned(),
            ));
        }
        Ok(())
    }

    /// Rebuilds the [`ProofRequest`] this vector describes, against the mock
    /// backend. Used by the runner's mock tier.
    pub fn to_request(&self) -> ProofRequest {
        let mut witness = PrivateWitnessBag::new();
        for (name, hex) in &self.witness.private {
            witness
                .insert(
                    name.clone(),
                    SecretValue::from_hex(hex).expect("vector hex validated on load"),
                )
                .expect("vector private names are unique");
        }
        ProofRequest::new(
            RequestId::new(format!("vector-{}", self.id.replace(['/', '\\'], "-"))),
            self.operation,
            self.circuit.clone(),
            self.circuit_version,
            Version::v0_1(),
            BackendId::new(BackendId::MOCK).expect("mock backend id is valid"),
            witness,
            self.witness.public.clone(),
            self.state_reference.clone(),
        )
    }
}

impl RawWitness {
    /// Converts raw private hex into canonical strings and the raw public bag
    /// into an ordered [`PublicInputBag`].
    fn into_witness(self, context: &str) -> Result<Witness, String> {
        let mut private = BTreeMap::new();
        for (name, hex) in self.private {
            let value = SecretValue::from_hex(&hex)
                .map_err(|e| format!("{context}: secret `{name}`: {e}"))?;
            private.insert(name, value.into_hex());
        }
        Ok(Witness {
            operation: self.operation,
            private,
            public: self.public.into_bag(&format!("{context}: public inputs"))?,
        })
    }
}

/// Discovers every vector JSON file under `root` (recursively).
pub fn discover(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    collect_json(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("cannot read test-vector directory `{}`: {e}", dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("cannot read entry in `{}`: {e}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_json(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
    Ok(())
}

/// Loads and validates every vector under `root`.
pub fn load_catalog(root: &Path) -> Result<Vec<TestVector>, String> {
    let mut vectors = Vec::new();
    for path in discover(root)? {
        vectors.push(TestVector::load(&path)?);
    }
    Ok(vectors)
}

/// The repo's test-vectors directory (relative to this crate's manifest).
pub fn catalog_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors")
}

/// Canonical state root used by state-bound fixtures (`ab`*32).
pub fn root_a() -> StateReference {
    StateReference::new(
        crucible_interfaces::RootDigest::from_hex(&"ab".repeat(32)).expect("valid hex"),
        1,
    )
}

/// A distinct root for replay/stale-state scenarios (`cd`*32).
pub fn root_b() -> StateReference {
    StateReference::new(
        crucible_interfaces::RootDigest::from_hex(&"cd".repeat(32)).expect("valid hex"),
        2,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_loads_with_valid_and_rejecting_vectors_per_operation() {
        let catalog = load_catalog(&catalog_root()).expect("catalog must load");
        assert!(!catalog.is_empty(), "catalog must not be empty");
        for op in [
            Operation::Register,
            Operation::Deposit,
            Operation::Merge,
            Operation::Transfer,
            Operation::Withdraw,
        ] {
            let ops: Vec<_> = catalog.iter().filter(|v| v.operation == op).collect();
            assert!(
                ops.iter().any(|v| v.category == "valid"),
                "{op} must have a valid vector"
            );
            assert!(
                ops.iter().any(|v| !v.expect_verification),
                "{op} must have a rejecting vector"
            );
        }
        // Ids are unique.
        let mut ids: Vec<_> = catalog.iter().map(|v| v.id.clone()).collect();
        ids.sort();
        let len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len, "vector ids must be unique");
    }

    #[test]
    fn valid_deposit_vector_reports_two_outputs() {
        let catalog = load_catalog(&catalog_root()).unwrap();
        let deposit = catalog
            .iter()
            .find(|v| v.id == "deposit-valid-001")
            .expect("deposit-valid-001 exists");
        assert!(deposit.expect_verification);
        assert_eq!(deposit.expected_public_outputs.len(), 2);
        assert!(
            deposit
                .expected_public_outputs
                .get("new_commitment")
                .is_some()
        );
        assert!(deposit.expected_public_outputs.get("nullifier").is_some());
    }

    #[test]
    fn invalid_vectors_still_form_valid_requests() {
        let catalog = load_catalog(&catalog_root()).unwrap();
        for vector in catalog.iter().filter(|v| !v.expect_verification) {
            // Rejecting vectors are *well-formed* requests: the backend must
            // reject them at witness-solve time, not because the request is
            // malformed.
            vector
                .to_request()
                .validate()
                .expect("request is structurally valid");
        }
    }

    #[test]
    fn non_canonical_hex_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.json");
        std::fs::write(
            &bad,
            r#"{
                "id": "bad-001",
                "operation": "register",
                "category": "valid",
                "circuit": "register",
                "circuit_version": "0.1.0",
                "witness": {
                    "operation": "register",
                    "private": { "account_sk": "0X1234" },
                    "public": { "entries": [["account_address", "abc"]] }
                },
                "expected_public_outputs": { "entries": [] },
                "state_reference": null,
                "expect_verification": true
            }"#,
        )
        .unwrap();
        let err = TestVector::load(&bad).expect_err("uppercase hex must be rejected");
        assert!(err.contains("account_sk"), "{err}");
    }
}
