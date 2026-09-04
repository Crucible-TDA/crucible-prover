//! Locating `nargo` and validating its version.

use std::path::PathBuf;
use std::process::Command;

use crate::errors::NoirError;
use crate::{MIN_NARGO_MAJOR, NARGO_BIN};

/// The parsed `nargo --version` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NargoVersion {
    /// Full version string as reported (e.g. `1.0.0-beta.26`).
    pub raw: String,
    /// Leading major component.
    pub major: u32,
}

/// A handle to the `nargo` binary.
///
/// Construction only *locates* the binary; the version check is explicit via
/// [`NoirToolchain::version`] so callers can decide whether a missing or
/// mismatched toolchain is fatal or skippable (tests, CI without proving).
#[derive(Debug, Clone)]
pub struct NoirToolchain {
    binary: PathBuf,
}

impl NoirToolchain {
    /// Locates `nargo` on `PATH`.
    ///
    /// Honors the `NARGO_BIN` environment variable override when set.
    pub fn locate() -> Result<NoirToolchain, NoirError> {
        let bin = std::env::var("NARGO_BIN").unwrap_or_else(|_| NARGO_BIN.to_owned());
        let path = which(&bin).ok_or(NoirError::BinaryNotFound)?;
        Ok(NoirToolchain { binary: path })
    }

    /// Whether a `nargo` binary is available on `PATH`.
    pub fn is_available() -> bool {
        NoirToolchain::locate().is_ok()
    }

    /// The resolved binary path.
    pub fn binary(&self) -> &PathBuf {
        &self.binary
    }

    /// Runs `nargo --version` and parses the reported version.
    pub fn version(&self) -> Result<NargoVersion, NoirError> {
        let output = Command::new(&self.binary)
            .arg("--version")
            .output()
            .map_err(|e| NoirError::Io {
                path: self.binary.display().to_string(),
                reason: e.to_string(),
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_version(&stdout)
    }

    /// Checks that the installed version is supported by this adapter.
    ///
    /// Returns `Ok(())` when the major version is supported, or
    /// [`NoirError::UnsupportedVersion`] otherwise.
    pub fn check_version(&self) -> Result<(), NoirError> {
        let version = self.version()?;
        if version.major < MIN_NARGO_MAJOR {
            return Err(NoirError::UnsupportedVersion {
                found: version.raw,
                supported: MIN_NARGO_MAJOR,
            });
        }
        Ok(())
    }
}

/// Finds an executable on `PATH` (Unix-style).
fn which(bin: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Parses `nargo --version` output like `nargo version = 1.0.0-beta.26`.
fn parse_version(stdout: &str) -> Result<NargoVersion, NoirError> {
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| NoirError::VersionParse("empty output".to_owned()))?;
    let raw = first
        .split_once('=')
        .map(|(_, v)| v.trim())
        .unwrap_or(first.trim());
    let raw = raw.trim();
    // Take everything up to the first non-version character.
    let digits: String = raw
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let major: u32 = digits
        .split('.')
        .next()
        .and_then(|m| m.parse().ok())
        .ok_or_else(|| NoirError::VersionParse(raw.to_owned()))?;
    Ok(NargoVersion {
        raw: raw.to_owned(),
        major,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_version_output() {
        let parsed = parse_version("nargo version = 1.0.0-beta.26\nnoirc version = ...\n").unwrap();
        assert_eq!(parsed.raw, "1.0.0-beta.26");
        assert_eq!(parsed.major, 1);
    }

    #[test]
    fn parses_bare_version_output() {
        let parsed = parse_version("0.36.0\n").unwrap();
        assert_eq!(parsed.major, 0);
    }

    #[test]
    fn rejects_garbage_version_output() {
        assert!(parse_version("").is_err());
        assert!(parse_version("nargo: command not found\n").is_err());
    }

    #[test]
    fn version_gate_accepts_supported_major() {
        let parsed = parse_version("nargo version = 1.0.0\n").unwrap();
        assert!(parsed.major >= MIN_NARGO_MAJOR);
    }
}
