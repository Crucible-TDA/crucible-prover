use std::fmt;

use serde::{Deserialize, Serialize};

/// The protocol operations whose validity proofs Crucible produces.
///
/// These mirror the Confidential Token operations of the underlying
/// architecture. The variant names are the canonical circuit identifiers:
/// a `Transfer` operation is proven by the `"transfer"` circuit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    /// Account registration.
    Register,
    /// Minting/adding confidential value to an account.
    Deposit,
    /// Consolidating multiple confidential commitments into one.
    Merge,
    /// Sending confidential value between accounts.
    Transfer,
    /// Redeeming confidential value out of the confidential domain.
    Withdraw,
}

impl Operation {
    /// Returns the canonical lowercase identifier of this operation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Operation::Register => "register",
            Operation::Deposit => "deposit",
            Operation::Merge => "merge",
            Operation::Transfer => "transfer",
            Operation::Withdraw => "withdraw",
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error parsing an unknown operation name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown operation `{0}`; expected one of: register, deposit, merge, transfer, withdraw")]
pub struct UnknownOperation(pub String);

impl std::str::FromStr for Operation {
    type Err = UnknownOperation;

    fn from_str(s: &str) -> Result<Operation, UnknownOperation> {
        match s {
            "register" => Ok(Operation::Register),
            "deposit" => Ok(Operation::Deposit),
            "merge" => Ok(Operation::Merge),
            "transfer" => Ok(Operation::Transfer),
            "withdraw" => Ok(Operation::Withdraw),
            other => Err(UnknownOperation(other.to_owned())),
        }
    }
}

/// Uniquely identifies a circuit in the Crucible catalog.
///
/// The identifier is an opaque, validated string. For the standard protocol
/// operations it equals [`Operation::as_str`], but custom/experimental
/// circuits may use any well-formed identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CircuitId(String);

impl CircuitId {
    /// Maximum length of a circuit identifier.
    pub const MAX_LEN: usize = 128;

    /// Returns the canonical circuit id for a protocol [`Operation`].
    pub fn for_operation(operation: Operation) -> CircuitId {
        CircuitId(operation.as_str().to_owned())
    }

    /// Validates and constructs a circuit identifier from a raw string.
    pub fn new(id: impl Into<String>) -> Result<CircuitId, CircuitIdError> {
        let id = id.into();
        if id.is_empty() || id.len() > Self::MAX_LEN {
            return Err(CircuitIdError::InvalidLength {
                actual: id.len(),
                max: Self::MAX_LEN,
            });
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(CircuitIdError::InvalidCharacters);
        }
        Ok(CircuitId(id))
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CircuitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Error constructing a [`CircuitId`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CircuitIdError {
    /// The identifier was empty or longer than [`CircuitId::MAX_LEN`].
    #[error("circuit id must be 1..={max} characters, got {actual}")]
    InvalidLength {
        /// The offending length.
        actual: usize,
        /// The maximum allowed length.
        max: usize,
    },
    /// The identifier contains characters outside `[A-Za-z0-9._-]`.
    #[error("circuit id contains invalid characters (allowed: alphanumeric, '-', '_', '.')")]
    InvalidCharacters,
}

/// Semantic version of a circuit or artifact.
///
/// Versions are explicit and mandatory on the wire: a proof is only valid for
/// the exact `(circuit, version)` tuple it was generated against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version {
    /// Breaking-change component.
    pub major: u32,
    /// Feature component.
    pub minor: u32,
    /// Patch component.
    pub patch: u32,
}

impl Version {
    /// Constructs a version from its three components.
    pub const fn new(major: u32, minor: u32, patch: u32) -> Version {
        Version {
            major,
            minor,
            patch,
        }
    }

    /// Convenience constructor for `0.1.0`.
    pub const fn v0_1() -> Version {
        Version::new(0, 1, 0)
    }

    /// Returns the version with the patch component incremented.
    pub const fn bump_patch(self) -> Version {
        Version::new(self.major, self.minor, self.patch + 1)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for Version {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Version, VersionParseError> {
        let mut parts = s.split('.');
        let (major, minor, patch) = match (parts.next(), parts.next(), parts.next()) {
            (Some(a), Some(b), Some(c)) if parts.next().is_none() => (a, b, c),
            _ => return Err(VersionParseError::Malformed),
        };
        let parse = |p: &str| p.parse::<u32>().map_err(|_| VersionParseError::Malformed);
        Ok(Version::new(parse(major)?, parse(minor)?, parse(patch)?))
    }
}

impl Serialize for Version {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Version, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse::<Version>().map_err(serde::de::Error::custom)
    }
}

/// Error parsing a [`Version`] from its `major.minor.patch` form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VersionParseError {
    /// The string was not three dot-separated non-negative integers.
    #[error("version must be formatted as major.minor.patch, e.g. 1.4.2")]
    Malformed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_round_trips_through_canonical_names() {
        for op in [
            Operation::Register,
            Operation::Deposit,
            Operation::Merge,
            Operation::Transfer,
            Operation::Withdraw,
        ] {
            assert_eq!(op.as_str().parse::<Operation>(), Ok(op));
            assert_eq!(op.to_string(), op.as_str());
            assert_eq!(CircuitId::for_operation(op).as_str(), op.as_str());
        }
        assert_eq!(
            "burn".parse::<Operation>(),
            Err(UnknownOperation("burn".into()))
        );
    }

    #[test]
    fn operation_serde_uses_lowercase_names() {
        let json = serde_json::to_string(&Operation::Transfer).unwrap();
        assert_eq!(json, "\"transfer\"");
        assert_eq!(
            serde_json::from_str::<Operation>("\"withdraw\"").unwrap(),
            Operation::Withdraw
        );
        assert!(serde_json::from_str::<Operation>("\"burn\"").is_err());
    }

    #[test]
    fn circuit_id_rejects_junk() {
        assert!(CircuitId::new("transfer").is_ok());
        assert!(CircuitId::new("").is_err());
        assert!(CircuitId::new("has space").is_err());
        assert!(CircuitId::new("sneaky\nnewline").is_err());
        assert!(CircuitId::new("x".repeat(129)).is_err());
        assert_eq!(
            CircuitId::new("x".repeat(129)).unwrap_err(),
            CircuitIdError::InvalidLength {
                actual: 129,
                max: 128
            }
        );
    }

    #[test]
    fn version_parses_and_serializes() {
        let v = "1.4.2".parse::<Version>().unwrap();
        assert_eq!(v, Version::new(1, 4, 2));
        assert_eq!(v.to_string(), "1.4.2");
        assert_eq!(
            serde_json::from_str::<Version>("\"0.1.0\"").unwrap(),
            Version::v0_1()
        );
        assert_eq!(Version::v0_1().bump_patch(), Version::new(0, 1, 1));
        assert!("1.4".parse::<Version>().is_err());
        assert!("a.b.c".parse::<Version>().is_err());
        assert!("1.4.2.3".parse::<Version>().is_err());
    }

    #[test]
    fn version_ordering_is_semantic() {
        assert!(Version::new(1, 0, 0) > Version::new(0, 9, 99));
        assert!(Version::new(1, 2, 0) > Version::new(1, 1, 99));
    }
}
