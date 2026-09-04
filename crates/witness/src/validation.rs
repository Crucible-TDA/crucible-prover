//! Structural validation of assembled witnesses.
//!
//! Per-operation *semantic* checks (does this amount exceed the balance, is
//! this owner authorized, …) live in the circuits and in
//! `crucible-scenarios`. What belongs here is the structural layer that is
//! true for every operation: witnesses must be non-empty where secrecy is
//! required, names must not collide across the public/private boundary, and
//! private values must never be exposed through the public side.

use crucible_interfaces::Operation;

use crate::builder::WitnessData;
use crate::errors::WitnessError;

/// Structural validation rules applied to every assembled witness.
pub struct WitnessValidation;

impl WitnessValidation {
    /// Validates `data`.
    ///
    /// - Transfer/Merge/Withdraw are state-bound operations and therefore
    ///   require at least one public binding value and at least one private
    ///   value (an empty private witness would prove nothing about secrets).
    /// - Register/Deposit are checked for emptiness of both sides as well:
    ///   a witness with neither public context nor secrets cannot describe a
    ///   meaningful transition.
    pub fn validate(data: &WitnessData) -> Result<(), WitnessError> {
        if data.public_inputs().is_empty() {
            return Err(WitnessError::MissingRequired {
                operation: data.operation(),
                side: crate::errors::WitnessSide::Public,
                name: "<any>".to_owned(),
            });
        }
        if data.private_count() == 0 {
            return Err(WitnessError::MissingRequired {
                operation: data.operation(),
                side: crate::errors::WitnessSide::Private,
                name: "<any>".to_owned(),
            });
        }
        // Cross-side name overlap is already prevented by the assembler;
        // re-check here so hand-rolled WitnessData (e.g. from fixtures)
        // cannot bypass it.
        for name in data.public_inputs().names() {
            if data.private().get(name).is_some() {
                return Err(WitnessError::Overlap {
                    name: name.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Convenience predicate for callers that only need a yes/no answer.
    pub fn is_valid(data: &WitnessData) -> bool {
        WitnessValidation::validate(data).is_ok()
    }
}

/// True when `operation` semantically requires state binding.
pub fn requires_state_binding(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Transfer | Operation::Merge | Operation::Withdraw
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::{FieldValue, SecretValue};

    fn data_with(public: bool, private: bool) -> WitnessData {
        let mut a = crate::builder::WitnessAssembler::for_operation(Operation::Transfer);
        if public {
            a = a
                .with_public("token", FieldValue::from_hex("01").unwrap())
                .unwrap();
        }
        if private {
            a = a
                .with_private("amount", SecretValue::from_hex("01").unwrap())
                .unwrap();
        }
        a.assemble().unwrap()
    }

    #[test]
    fn requires_both_sides() {
        assert!(WitnessValidation::validate(&data_with(true, true)).is_ok());
        assert!(WitnessValidation::validate(&data_with(true, false)).is_err());
        assert!(WitnessValidation::validate(&data_with(false, true)).is_err());
    }

    #[test]
    fn state_binding_requirements_match_operations() {
        assert!(requires_state_binding(Operation::Transfer));
        assert!(requires_state_binding(Operation::Withdraw));
        assert!(requires_state_binding(Operation::Merge));
        assert!(!requires_state_binding(Operation::Register));
        assert!(!requires_state_binding(Operation::Deposit));
    }
}
