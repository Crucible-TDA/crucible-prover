//! Proof envelope assembly.
//!
//! Once a provider returns a [`ProofResponse`], the service wraps it into the
//! versioned wire form — the [`ProofEnvelope`] — so the proof is traceable
//! and serializable. These helpers are the single path from response to
//! envelope.

use crucible_interfaces::{Operation, ProofResponse};

use crate::errors::CoreError;

/// Wraps a proof response into the current envelope format.
///
/// `operation` is taken from the request that produced the response; it is
/// the *semantic* operation, which may differ from the circuit id for custom
/// circuits. `produced_by` names the producing service (e.g.
/// `crucible-prover/0.1.0`) for provenance.
pub fn assemble_envelope(
    response: &ProofResponse,
    operation: Operation,
    produced_by: impl Into<String>,
) -> Result<crucible_proof_types::ProofEnvelope, CoreError> {
    Ok(crucible_proof_types::ProofEnvelope::from_response(
        response,
        operation,
        produced_by,
    ))
}

/// Serializes a proof response into the canonical envelope JSON.
///
/// This is the storage/exchange representation of a generated proof.
pub fn serialized_envelope(
    response: &ProofResponse,
    operation: Operation,
    produced_by: impl Into<String>,
) -> Result<String, CoreError> {
    let envelope = assemble_envelope(response, operation, produced_by)?;
    envelope
        .to_json()
        .map_err(|e| CoreError::Envelope(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::ProofProvider;
    use crucible_mock::{MockProver, fixtures};

    #[test]
    fn response_round_trips_through_envelope_json() {
        let request = fixtures::transfer_request();
        let response = MockProver::new().generate(&request).unwrap();
        let json = serialized_envelope(&response, request.operation, "test/0.1.0").unwrap();
        let parsed = crucible_proof_types::ProofEnvelope::from_json(&json).unwrap();
        assert_eq!(parsed.circuit, response.circuit);
        assert_eq!(parsed.circuit_version, response.circuit_version);
        assert_eq!(parsed.operation, Operation::Transfer);
        assert_eq!(parsed.public_outputs, response.public_outputs);
        assert_eq!(parsed.state_reference, response.state_reference);
    }

    #[test]
    fn envelope_keeps_operation_from_the_request() {
        // A custom circuit id must not leak the wrong operation label.
        let request = fixtures::withdraw_request();
        let response = MockProver::new().generate(&request).unwrap();
        let envelope = assemble_envelope(&response, request.operation, "test").unwrap();
        assert_eq!(envelope.operation, Operation::Withdraw);
        assert_eq!(envelope.circuit.as_str(), "withdraw");
    }
}
