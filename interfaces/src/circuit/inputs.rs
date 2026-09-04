use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Error validating a field value, name, or bag.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FieldError {
    /// A value was not canonical hex.
    #[error("field value must be canonical lowercase hex, got: {reason}")]
    InvalidHex {
        /// Machine-readable reason for the rejection.
        reason: &'static str,
    },
    /// A name was empty or too long.
    #[error("field name must be 1..={max} characters")]
    InvalidNameLength {
        /// The maximum allowed name length.
        max: usize,
    },
    /// A name contains characters outside `[a-z0-9_]`.
    #[error("field name may only contain lowercase alphanumerics and underscores")]
    InvalidNameCharacters,
    /// A name appeared twice in the same bag.
    #[error("duplicate field name: {name}")]
    DuplicateName {
        /// The duplicated name.
        name: String,
    },
}

/// A public (non-secret) value bound to a circuit input or output.
///
/// Values are serialized as canonical lowercase hexadecimal without a `0x`
/// prefix and without leading zeroes (`0` is written as `"0"`). This is the
/// stable textual form used for binding hashes, test vectors, and public
/// inputs.
///
/// Note: field *arithmetic* (e.g. modulus checks) happens inside the Noir
/// circuits. This type only guarantees a canonical textual representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldValue(String);

impl FieldValue {
    /// The maximum number of hex characters accepted (a 256-bit digest).
    pub const MAX_HEX_LEN: usize = 64;

    /// The additive identity.
    pub fn zero() -> FieldValue {
        FieldValue("0".to_owned())
    }

    /// Validates and canonicalizes a hexadecimal value.
    pub fn from_hex(hex: &str) -> Result<FieldValue, FieldError> {
        let s = hex.strip_prefix("0x").unwrap_or(hex);
        if s.is_empty() {
            return Err(FieldError::InvalidHex { reason: "empty" });
        }
        if s.len() > Self::MAX_HEX_LEN {
            return Err(FieldError::InvalidHex {
                reason: "longer than 64 hex chars",
            });
        }
        if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(FieldError::InvalidHex { reason: "non-hex" });
        }
        let lower = s.to_ascii_lowercase();
        let trimmed = lower.trim_start_matches('0');
        let canonical = if trimmed.is_empty() {
            "0".to_owned()
        } else {
            trimmed.to_owned()
        };
        Ok(FieldValue(canonical))
    }

    /// Returns the canonical lowercase hex representation.
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// Returns `true` if this is the zero element.
    pub fn is_zero(&self) -> bool {
        self.0 == "0"
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for FieldValue {
    type Err = FieldError;

    fn from_str(s: &str) -> Result<FieldValue, FieldError> {
        FieldValue::from_hex(s)
    }
}

impl Serialize for FieldValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for FieldValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<FieldValue, D::Error> {
        let raw = String::deserialize(deserializer)?;
        FieldValue::from_hex(&raw).map_err(serde::de::Error::custom)
    }
}

/// A private value that must never leave the proving boundary unprotected.
///
/// `SecretValue` deliberately implements **no** `Debug`, `Display`, or
/// `Serialize`. It cannot be formatted, logged, or JSON-encoded by accident;
/// the only way to obtain its contents is the explicit, opt-in
/// [`SecretValue::into_hex`].
///
/// Treat every `SecretValue` as a field element of the Noir circuit that
/// consumes it: secret randomness, a secret opening, a private amount.
pub struct SecretValue(Box<str>);

impl SecretValue {
    /// Validates and wraps a private hexadecimal value.
    ///
    /// The same canonicalization rules as [`FieldValue::from_hex`] apply so
    /// witness values and public values share one textual format.
    pub fn from_hex(hex: &str) -> Result<SecretValue, FieldError> {
        let canonical = FieldValue::from_hex(hex)?;
        Ok(SecretValue(canonical.as_hex().into()))
    }

    /// Consumes the secret and returns its canonical hex contents.
    ///
    /// This is the single, explicit escape hatch. It exists so the witness
    /// encoder can hand values to the prover backend; it must never be used
    /// for logging, error messages, or fixtures.
    pub fn into_hex(self) -> String {
        self.0.into()
    }
}

impl Clone for SecretValue {
    fn clone(&self) -> SecretValue {
        SecretValue(self.0.clone())
    }
}

impl PartialEq for SecretValue {
    fn eq(&self, other: &SecretValue) -> bool {
        self.0 == other.0
    }
}

impl Eq for SecretValue {}

fn validate_name(name: &str) -> Result<(), FieldError> {
    if name.is_empty() || name.len() > 64 {
        return Err(FieldError::InvalidNameLength { max: 64 });
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        return Err(FieldError::InvalidNameCharacters);
    }
    Ok(())
}

/// A private (secret) named value.
type SecretEntry = (String, SecretValue);

/// Ordered collection of private witness values.
///
/// This is where private information enters the proving system, so the type
/// is deliberately minimal: it can be constructed, queried by name, and
/// consumed by the encoder — but it never formats, logs, or serializes its
/// contents, and it refuses duplicate names so two circuit inputs can never
/// silently alias.
#[derive(Clone)]
pub struct PrivateWitnessBag {
    entries: Vec<SecretEntry>,
}

impl PrivateWitnessBag {
    /// Creates an empty bag.
    pub fn new() -> PrivateWitnessBag {
        PrivateWitnessBag {
            entries: Vec::new(),
        }
    }

    /// Inserts a secret under `name`, rejecting duplicates.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        value: SecretValue,
    ) -> Result<(), FieldError> {
        let name = name.into();
        validate_name(&name)?;
        if self.entries.iter().any(|(n, _)| n == &name) {
            return Err(FieldError::DuplicateName { name });
        }
        self.entries.push((name, value));
        Ok(())
    }

    /// Returns the secret stored under `name`, if any.
    pub fn get(&self, name: &str) -> Option<&SecretValue> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// Names of all private values, in insertion order. Values are never
    /// exposed through this iterator.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(n, _)| n.as_str())
    }

    /// Number of private values.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the bag is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Consumes the bag, returning its private contents to the caller.
    ///
    /// Intended for the witness encoder that serializes values for a prover
    /// backend. Callers are responsible for never logging or storing the
    /// returned material.
    pub fn into_entries(self) -> Vec<(String, SecretValue)> {
        self.entries
    }
}

impl Default for PrivateWitnessBag {
    fn default() -> Self {
        PrivateWitnessBag::new()
    }
}

impl fmt::Debug for PrivateWitnessBag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redacted by construction: names only, never values.
        f.debug_struct("PrivateWitnessBag")
            .field("entries", &self.names().collect::<Vec<_>>())
            .finish()
    }
}

/// A public (non-secret) named field value.
type PublicEntry = (String, FieldValue);

/// Ordered collection of public circuit inputs or outputs.
///
/// Public values are safe to log, serialize, and include in fixtures; they
/// are the values a proof commits to and a verifier checks against.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PublicInputBag {
    entries: Vec<PublicEntry>,
}

impl PublicInputBag {
    /// Creates an empty bag.
    pub fn new() -> PublicInputBag {
        PublicInputBag {
            entries: Vec::new(),
        }
    }

    /// Inserts a public value under `name`, rejecting duplicates.
    pub fn insert(&mut self, name: impl Into<String>, value: FieldValue) -> Result<(), FieldError> {
        let name = name.into();
        validate_name(&name)?;
        if self.entries.iter().any(|(n, _)| n == &name) {
            return Err(FieldError::DuplicateName { name });
        }
        self.entries.push((name, value));
        Ok(())
    }

    /// Returns the public value stored under `name`, if any.
    pub fn get(&self, name: &str) -> Option<&FieldValue> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// Iterates over `(name, value)` pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &FieldValue)> {
        self.entries.iter().map(|(n, v)| (n.as_str(), v))
    }

    /// Names of all public values, in insertion order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(n, _)| n.as_str())
    }

    /// Number of public values.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the bag is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Canonical byte encoding of the bag contents.
    ///
    /// Produces `name=value` lines in insertion order, one per entry. This is
    /// the input to binding hashes so that any change to a name or value —
    /// or their order — changes the resulting digest.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, value) in &self.entries {
            out.extend_from_slice(name.as_bytes());
            out.push(b'=');
            out.extend_from_slice(value.as_hex().as_bytes());
            out.push(b'\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_values_are_canonicalized() {
        assert_eq!(FieldValue::from_hex("0x00abc").unwrap().as_hex(), "abc");
        assert_eq!(FieldValue::from_hex("ABC").unwrap().as_hex(), "abc");
        assert_eq!(FieldValue::from_hex("0000").unwrap().as_hex(), "0");
        assert!(FieldValue::from_hex("").is_err());
        assert!(FieldValue::from_hex("0x").is_err());
        assert!(FieldValue::from_hex("zz").is_err());
        assert!(FieldValue::from_hex(&"1".repeat(65)).is_err());
        assert!(FieldValue::zero().is_zero());
    }

    #[test]
    fn field_value_serde_round_trips_and_validates() {
        let v = FieldValue::from_hex("0x00DEADbeef").unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"deadbeef\"");
        let back: FieldValue = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
        assert!(serde_json::from_str::<FieldValue>("\"not-hex!\"").is_err());
    }

    #[test]
    fn public_bag_rejects_duplicates_and_bad_names() {
        let mut bag = PublicInputBag::new();
        bag.insert("sender", FieldValue::from_hex("1").unwrap())
            .unwrap();
        assert!(bag.insert("sender", FieldValue::zero()).is_err());
        assert!(bag.insert("has space", FieldValue::zero()).is_err());
        assert!(bag.insert("UPPER", FieldValue::zero()).is_err());
        assert!(bag.insert("", FieldValue::zero()).is_err());
        assert_eq!(bag.len(), 1);
        assert_eq!(bag.get("sender").unwrap().as_hex(), "1");
        assert!(bag.get("recipient").is_none());
    }

    #[test]
    fn canonical_bytes_are_order_and_value_sensitive() {
        let mut a = PublicInputBag::new();
        a.insert("x", FieldValue::from_hex("1").unwrap()).unwrap();
        a.insert("y", FieldValue::from_hex("2").unwrap()).unwrap();
        let mut b = PublicInputBag::new();
        b.insert("y", FieldValue::from_hex("2").unwrap()).unwrap();
        b.insert("x", FieldValue::from_hex("1").unwrap()).unwrap();
        let mut c = PublicInputBag::new();
        c.insert("x", FieldValue::from_hex("2").unwrap()).unwrap();
        c.insert("y", FieldValue::from_hex("1").unwrap()).unwrap();

        assert_ne!(a.canonical_bytes(), b.canonical_bytes());
        assert_ne!(a.canonical_bytes(), c.canonical_bytes());
        assert_eq!(a.canonical_bytes(), a.canonical_bytes());
    }

    #[test]
    fn secret_values_never_reach_debug_or_serde() {
        let secret = SecretValue::from_hex("0x00c0ffee").unwrap();
        assert!(secret.clone() == secret);

        let mut bag = PrivateWitnessBag::new();
        bag.insert("amount", SecretValue::from_hex("1234").unwrap())
            .unwrap();
        assert!(
            bag.insert("amount", SecretValue::from_hex("1").unwrap())
                .is_err()
        );
        assert_eq!(bag.len(), 1);
        assert!(bag.get("amount").is_some());
        assert_eq!(bag.names().collect::<Vec<_>>(), vec!["amount"]);

        let debug = format!("{bag:?}");
        assert!(
            !debug.contains("1234"),
            "debug output leaked a secret: {debug}"
        );
        assert!(debug.contains("amount"));

        // The only escape hatch is explicit consumption.
        let entries = bag.clone().into_entries();
        assert_eq!(entries[0].1.clone().into_hex(), "1234");
    }

    #[test]
    fn secret_and_public_values_share_canonical_hex_rules() {
        assert_eq!(SecretValue::from_hex("0x0A").unwrap().into_hex(), "a");
        assert!(SecretValue::from_hex("gg").is_err());
        assert!(SecretValue::from_hex("").is_err());
    }
}
