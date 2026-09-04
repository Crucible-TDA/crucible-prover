//! Encoding witnesses for the Noir toolchain.
//!
//! Noir consumes witness values through `Prover.toml` files of the form
//!
//! ```toml
//! name = "0x…"
//! ```
//!
//! This module is the *single intentional escape hatch* where private values
//! leave memory. Encoding happens here and only here, and the resulting
//! output is meant for a prover backend or a file with restrictive
//! permissions — never for logs, errors, or fixtures.

use std::io::Write;
use std::path::Path;

use crate::builder::WitnessData;
use crate::errors::WitnessError;

/// Encodes witness data into the Noir `Prover.toml` text layout.
///
/// Output is deterministic for identical input: public values first, then
/// private values, each in bag insertion order. Values are written as
/// `0x`-prefixed hex — Noir's witness parser treats an unprefixed string as
/// **decimal**, so a bare `ab` fails to parse and a bare `1234` silently
/// means decimal 1234 rather than `0x1234`. Callers must treat the returned
/// string as secret material.
pub fn encode_toml(data: &WitnessData) -> String {
    let mut out = String::new();
    for (name, value) in data.public_inputs().iter() {
        out.push_str(&format!("{name} = \"0x{}\"\n", value.as_hex()));
    }
    for name in data.private().names() {
        // Values are pulled out only here, never formatted.
        let value = data.private().get(name).expect("name from iterator");
        out.push_str(&format!("{name} = \"0x{}\"\n", value.clone().into_hex()));
    }
    out
}

/// Writes a witness file with restrictive permissions.
///
/// On Unix the file is created with mode `0600` so private witness material
/// is not world-readable even if the directory is loose. Existing files are
/// truncated.
pub fn write_prover_toml(data: &WitnessData, path: &Path) -> Result<(), WitnessError> {
    let contents = encode_toml(data);
    write_restricted(path, contents.as_bytes())
}

/// Shared restricted write used by all witness outputs.
pub(crate) fn write_restricted(path: &Path, contents: &[u8]) -> Result<(), WitnessError> {
    let mut file = open_for_witness(path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(())
}

/// Opens a witness output file, creating it with `0600` permissions on Unix.
#[cfg(unix)]
fn open_for_witness(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true).mode(0o600);
    options.open(path)
}

/// Non-Unix fallback (best-effort restrictive permissions).
#[cfg(not(unix))]
fn open_for_witness(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::{FieldValue, Operation, SecretValue};
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn sample_data() -> WitnessData {
        crate::builder::WitnessAssembler::for_operation(Operation::Transfer)
            .with_public("token", FieldValue::from_hex("01").unwrap())
            .unwrap()
            .with_private("amount", SecretValue::from_hex("0x0000ab").unwrap())
            .unwrap()
            .assemble()
            .unwrap()
    }

    #[test]
    fn encoding_is_deterministic_and_well_formed() {
        let a = encode_toml(&sample_data());
        let b = encode_toml(&sample_data());
        assert_eq!(a, b);
        assert!(a.contains("token = \"0x1\"\n"), "unexpected: {a}");
        assert!(a.contains("amount = \"0xab\"\n"), "unexpected: {a}");
        // Values whose hex contains letters must stay hex, not become decimal.
        assert!(!a.contains("amount = \"ab\""), "bare hex would parse as decimal: {a}");
        // Public values come before private ones.
        assert!(a.find("token").unwrap() < a.find("amount").unwrap());
    }

    #[test]
    fn witness_file_is_written_with_restrictive_permissions() {
        let dir = tempdir().unwrap();
        let path: PathBuf = dir.path().join("Prover.toml");
        write_prover_toml(&sample_data(), &path).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "witness file must be 0600");
        }
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("amount = \"0xab\""));
    }
}
