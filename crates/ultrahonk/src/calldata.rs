//! Calldata encoding for on-chain UltraHonk verification.
//!
//! An UltraHonk verifier on Soroban receives the proof plus the public
//! inputs the proof commits to. The *bytes* of those public inputs must be in
//! exactly the order and width the deployed verifier contract expects —
//! getting this wrong is a silent "verification failed" on-chain, which is
//! precisely the kind of local/on-chain discrepancy Crucible exists to catch.
//!
//! # What this module provides
//!
//! A deterministic, versioned [`CalldataEncoder`] that packs the circuit's
//! public inputs — in ABI declaration order, as 32-byte big-endian field
//! elements — into one byte string prefixed by a format version and the
//! input count. The decoder round-trips it, and the encoder enforces that
//! inputs are supplied in exactly the declared order.
//!
//! # Calibration warning
//!
//! The on-chain verifier's exact byte layout is defined by the deployed
//! verifier contract, which lands with the circuits/Soroban batch. Until
//! then, this encoding is the *candidate* layout and must be validated
//! against the real verifier before production use. The format version tag
//! exists so that calibration can land as a new version without breaking
//! stored fixtures.

use crucible_interfaces::{FieldError, FieldValue, OutputBag};

use crate::errors::UltraHonkError;

/// Byte width of one encoded field element.
pub const FIELD_WIDTH: usize = 32;

/// Format version byte for the current calldata layout.
pub const CALDATA_VERSION: u8 = 1;

/// One ordered public input: its circuit name and value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicInput {
    /// Input name (for diagnostics and ordering checks).
    pub name: String,
    /// Input value as canonical hex.
    pub value: FieldValue,
}

impl PublicInput {
    /// Creates a public input.
    pub fn new(name: impl Into<String>, value: FieldValue) -> PublicInput {
        PublicInput {
            name: name.into(),
            value,
        }
    }
}

/// Encodes public inputs into the calldata byte layout for an on-chain
/// UltraHonk verifier.
#[derive(Debug, Clone, Copy, Default)]
pub struct CalldataEncoder;

/// The decoded form of calldata public inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedCalldata {
    /// Format version.
    pub version: u8,
    /// Public inputs in the order they were encoded.
    pub inputs: Vec<PublicInput>,
}

impl CalldataEncoder {
    /// Encodes a public input list in ABI order.
    ///
    /// Layout: `[version:1][count:4 LE][input:32 BE]...`
    pub fn encode(&self, inputs: &[PublicInput]) -> Result<Vec<u8>, UltraHonkError> {
        let mut out = Vec::with_capacity(1 + 4 + inputs.len() * FIELD_WIDTH);
        out.push(CALDATA_VERSION);
        out.extend_from_slice(&(inputs.len() as u32).to_le_bytes());
        for input in inputs {
            let bytes = encode_field(&input.value, &input.name)?;
            out.extend_from_slice(&bytes);
        }
        Ok(out)
    }

    /// Encodes the contents of an [`OutputBag`], using insertion order as the
    /// ABI order.
    pub fn encode_bag(&self, bag: &OutputBag) -> Result<Vec<u8>, UltraHonkError> {
        let inputs: Vec<PublicInput> = bag
            .iter()
            .map(|(name, value)| PublicInput::new(name, value.clone()))
            .collect();
        self.encode(&inputs)
    }

    /// Decodes calldata back into ordered public inputs.
    pub fn decode(&self, bytes: &[u8]) -> Result<DecodedCalldata, UltraHonkError> {
        if bytes.len() < 5 {
            return Err(UltraHonkError::Truncated {
                expected: 0,
                actual: 0,
            });
        }
        let version = bytes[0];
        let count = u32::from_le_bytes(bytes[1..5].try_into().expect("4 bytes")) as usize;
        let payload = &bytes[5..];
        if payload.len() != count * FIELD_WIDTH {
            return Err(UltraHonkError::Truncated {
                expected: count * FIELD_WIDTH,
                actual: payload.len(),
            });
        }
        let mut inputs = Vec::with_capacity(count);
        for (index, chunk) in payload.as_chunks::<FIELD_WIDTH>().0.iter().enumerate() {
            // Field values are unnamed on the wire; name them positionally.
            let hex = hex::encode(chunk);
            let value =
                FieldValue::from_hex(&hex).map_err(|e: FieldError| UltraHonkError::Encode {
                    name: format!("input[{index}]"),
                    reason: e.to_string(),
                })?;
            inputs.push(PublicInput::new(format!("input[{index}]"), value));
        }
        Ok(DecodedCalldata { version, inputs })
    }
}

/// Encodes one field value as 32-byte big-endian, rejecting values that
/// exceed the width.
fn encode_field(value: &FieldValue, name: &str) -> Result<[u8; 32], UltraHonkError> {
    let hex = value.as_hex();
    // Canonical hex has no leading zeros, so it can be shorter than 64 chars;
    // pad on the left to the 32-byte width.
    if hex.len() > 64 {
        return Err(UltraHonkError::Encode {
            name: name.to_owned(),
            reason: "value exceeds 32 bytes".to_owned(),
        });
    }
    let mut out = [0u8; 32];
    let padded = format!("{:0>64}", hex);
    let decoded = hex::decode(&padded).expect("validated hex is decodable");
    out.copy_from_slice(&decoded);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_inputs() -> Vec<PublicInput> {
        vec![
            PublicInput::new("token", FieldValue::from_hex("01").unwrap()),
            PublicInput::new("amount", FieldValue::from_hex("2a").unwrap()),
            PublicInput::new("commitment", FieldValue::from_hex("c0ffee").unwrap()),
        ]
    }

    #[test]
    fn encodes_with_version_and_count_prefix() {
        let bytes = CalldataEncoder.encode(&sample_inputs()).unwrap();
        assert_eq!(bytes.len(), 1 + 4 + 3 * 32);
        assert_eq!(bytes[0], CALDATA_VERSION);
        assert_eq!(u32::from_le_bytes(bytes[1..5].try_into().unwrap()), 3);
        // First field value 0x01 → last byte is 1.
        assert_eq!(bytes[5 + 31], 0x01);
    }

    #[test]
    fn round_trips_through_decode() {
        let inputs = sample_inputs();
        let bytes = CalldataEncoder.encode(&inputs).unwrap();
        let decoded = CalldataEncoder.decode(&bytes).unwrap();
        assert_eq!(decoded.version, CALDATA_VERSION);
        // Values survive; names become positional.
        assert_eq!(decoded.inputs.len(), 3);
        assert_eq!(decoded.inputs[0].value, inputs[0].value);
        assert_eq!(decoded.inputs[1].value, inputs[1].value);
        assert_eq!(decoded.inputs[2].value.as_hex(), "c0ffee");
    }

    #[test]
    fn bag_encoding_uses_insertion_order() {
        let mut bag = OutputBag::new();
        bag.insert("sender", FieldValue::from_hex("aa").unwrap())
            .unwrap();
        bag.insert("amount", FieldValue::from_hex("bb").unwrap())
            .unwrap();
        let bytes = CalldataEncoder.encode_bag(&bag).unwrap();
        assert_eq!(u32::from_le_bytes(bytes[1..5].try_into().unwrap()), 2);
        assert_eq!(bytes[5 + 31], 0xaa); // sender first
        assert_eq!(bytes[5 + 32 + 31], 0xbb); // amount second
    }

    #[test]
    fn deterministic_encoding() {
        let inputs = sample_inputs();
        assert_eq!(
            CalldataEncoder.encode(&inputs).unwrap(),
            CalldataEncoder.encode(&inputs).unwrap()
        );
    }

    #[test]
    fn truncated_calldata_is_rejected() {
        let bytes = CalldataEncoder.encode(&sample_inputs()).unwrap();
        let truncated = &bytes[..bytes.len() - 10];
        assert!(CalldataEncoder.decode(truncated).is_err());
        assert!(CalldataEncoder.decode(&[]).is_err());
    }
}
