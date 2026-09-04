//! Witness assembly: combining public inputs and private witness values into
//! one validated [`WitnessData`].

use std::fmt;

use crucible_interfaces::{Operation, PrivateWitnessBag, ProofRequest, PublicInputBag};

use crate::errors::{WitnessError, WitnessSide};

/// The complete witness for one operation: its private values and the public
/// context it is bound to.
///
/// # Privacy
///
/// `Debug` is redacted by construction: private values are never printed.
/// There is no `Serialize` implementation — serializing a witness is an
/// explicit, intentional act performed by the [`encoder`](crate::encoder).
#[derive(Clone)]
pub struct WitnessData {
    operation: Operation,
    public: PublicInputBag,
    private: PrivateWitnessBag,
}

impl WitnessData {
    /// Builds witness data from the request that carries it.
    pub fn from_request(request: &ProofRequest) -> WitnessData {
        WitnessData {
            operation: request.operation,
            public: request.public_inputs.clone(),
            private: request.witness.clone(),
        }
    }

    /// The operation this witness belongs to.
    pub fn operation(&self) -> Operation {
        self.operation
    }

    /// The public context values.
    pub fn public_inputs(&self) -> &PublicInputBag {
        &self.public
    }

    /// The private values (query by name only; see [`PrivateWitnessBag`]).
    pub fn private(&self) -> &PrivateWitnessBag {
        &self.private
    }

    /// Number of private values.
    pub fn private_count(&self) -> usize {
        self.private.len()
    }
}

impl fmt::Debug for WitnessData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WitnessData")
            .field("operation", &self.operation)
            .field("public", &self.public)
            .field("private_names", &self.private.names().collect::<Vec<_>>())
            .field("private_count", &self.private.len())
            .finish()
    }
}

/// Configurable witness assembler.
///
/// The assembler combines a public bag and a private bag into a
/// [`WitnessData`] and enforces structural rules before anything can be
/// encoded:
///
/// - names on the two sides must not overlap;
/// - every required name (per side) must be present.
///
/// Required names are caller-supplied because the exact circuit interface is
/// defined by each circuit (see `circuits/`); the assembler stays generic.
pub struct WitnessAssembler {
    operation: Operation,
    public: PublicInputBag,
    private: PrivateWitnessBag,
    required_public: Vec<String>,
    required_private: Vec<String>,
}

impl WitnessAssembler {
    /// Starts an assembler for `operation`.
    pub fn for_operation(operation: Operation) -> WitnessAssembler {
        WitnessAssembler {
            operation,
            public: PublicInputBag::new(),
            private: PrivateWitnessBag::new(),
            required_public: Vec::new(),
            required_private: Vec::new(),
        }
    }

    /// Sets the operation to assemble (fluent override).
    pub fn operation(mut self, operation: Operation) -> WitnessAssembler {
        self.operation = operation;
        self
    }

    /// Adds a public input value.
    pub fn with_public(
        mut self,
        name: impl Into<String>,
        value: crucible_interfaces::FieldValue,
    ) -> Result<WitnessAssembler, WitnessError> {
        self.public.insert(name, value)?;
        Ok(self)
    }

    /// Adds a private witness value.
    pub fn with_private(
        mut self,
        name: impl Into<String>,
        value: crucible_interfaces::SecretValue,
    ) -> Result<WitnessAssembler, WitnessError> {
        self.private.insert(name, value)?;
        Ok(self)
    }

    /// Declares a public name that must be present at assembly time.
    pub fn requires_public(mut self, name: impl Into<String>) -> WitnessAssembler {
        self.required_public.push(name.into());
        self
    }

    /// Declares a private name that must be present at assembly time.
    pub fn requires_private(mut self, name: impl Into<String>) -> WitnessAssembler {
        self.required_private.push(name.into());
        self
    }

    /// Validates and assembles the witness data.
    pub fn assemble(self) -> Result<WitnessData, WitnessError> {
        for name in &self.required_public {
            if self.public.get(name).is_none() {
                return Err(WitnessError::MissingRequired {
                    operation: self.operation,
                    side: WitnessSide::Public,
                    name: name.clone(),
                });
            }
        }
        for name in &self.required_private {
            if self.private.get(name).is_none() {
                return Err(WitnessError::MissingRequired {
                    operation: self.operation,
                    side: WitnessSide::Private,
                    name: name.clone(),
                });
            }
        }
        for name in self.public.names() {
            if self.private.get(name).is_some() {
                return Err(WitnessError::Overlap {
                    name: name.to_owned(),
                });
            }
        }
        Ok(WitnessData {
            operation: self.operation,
            public: self.public,
            private: self.private,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::{FieldValue, SecretValue};

    fn sample_secret(name: &str) -> Result<(String, SecretValue), WitnessError> {
        Ok((name.to_owned(), SecretValue::from_hex("0x10").unwrap()))
    }

    #[test]
    fn assembler_builds_valid_witness() {
        let (sname, svalue) = sample_secret("amount").unwrap();
        let data = WitnessAssembler::for_operation(Operation::Transfer)
            .with_public("token", FieldValue::from_hex("01").unwrap())
            .unwrap()
            .with_private(sname, svalue)
            .unwrap()
            .requires_public("token")
            .requires_private("amount")
            .assemble()
            .unwrap();
        assert_eq!(data.operation(), Operation::Transfer);
        assert_eq!(data.private_count(), 1);
        assert_eq!(data.public_inputs().len(), 1);
    }

    #[test]
    fn assembler_rejects_missing_required_names() {
        let (sname, svalue) = sample_secret("amount").unwrap();
        let err = WitnessAssembler::for_operation(Operation::Transfer)
            .with_public("token", FieldValue::from_hex("01").unwrap())
            .unwrap()
            .with_private(sname, svalue)
            .unwrap()
            .requires_public("token")
            .requires_private("missing_secret")
            .assemble()
            .unwrap_err();
        assert!(matches!(
            err,
            WitnessError::MissingRequired {
                operation: Operation::Transfer,
                side: WitnessSide::Private,
                ..
            }
        ));
    }

    #[test]
    fn assembler_rejects_public_private_overlap() {
        let err = WitnessAssembler::for_operation(Operation::Deposit)
            .with_public("token", FieldValue::from_hex("01").unwrap())
            .unwrap()
            .with_private("token", SecretValue::from_hex("01").unwrap())
            .unwrap()
            .assemble()
            .unwrap_err();
        assert!(matches!(err, WitnessError::Overlap { ref name } if name == "token"));
    }

    #[test]
    fn debug_view_never_prints_secret_values() {
        let data = WitnessAssembler::for_operation(Operation::Transfer)
            .with_public("token", FieldValue::from_hex("01").unwrap())
            .unwrap()
            .with_private("amount", SecretValue::from_hex("1234deadbeef").unwrap())
            .unwrap()
            .assemble()
            .unwrap();
        let debug = format!("{data:?}");
        assert!(!debug.contains("1234deadbeef"), "leaked: {debug}");
        assert!(debug.contains("amount"));
    }
}
