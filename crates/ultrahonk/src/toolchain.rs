//! Locating `bb` (Barretenberg) and validating its version.
//!
//! Proving and verification run in the Barretenberg binary, never inside
//! this crate. All interaction with `bb` therefore starts here: locating the
//! binary and confirming it is a version this adapter understands.

use std::path::PathBuf;
use std::process::Command;

use crate::errors::UltraHonkError;
use crate::{BB_BIN, MIN_BB_MAJOR};

/// The parsed `bb --version` output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BbVersion {
    /// Full version string as reported (e.g. `6.0.0-nightly.20260903`).
    pub raw: String,
    /// Leading major component.
    pub major: u32,
}

/// A handle to the `bb` binary.
///
/// Construction only *locates* the binary; the version check is explicit via
/// [`BbToolchain::version`] so callers can decide whether a missing or
/// mismatched toolchain is fatal or skippable (tests, CI without proving).
#[derive(Debug, Clone)]
pub struct BbToolchain {
    binary: PathBuf,
}

impl BbToolchain {
    /// Locates `bb` on `PATH`.
    ///
    /// Honors the `BB_BIN` environment variable override when set.
    pub fn locate() -> Result<BbToolchain, UltraHonkError> {
        let bin = std::env::var("BB_BIN").unwrap_or_else(|_| BB_BIN.to_owned());
        let path = which(&bin).ok_or(UltraHonkError::BinaryNotFound)?;
        Ok(BbToolchain { binary: path })
    }

    /// Whether a `bb` binary is available on `PATH`.
    pub fn is_available() -> bool {
        BbToolchain::locate().is_ok()
    }

    /// The resolved binary path.
    pub fn binary(&self) -> &PathBuf {
        &self.binary
    }

    /// Runs `bb --version` and parses the reported version.
    pub fn version(&self) -> Result<BbVersion, UltraHonkError> {
        let output = Command::new(&self.binary)
            .arg("--version")
            .output()
            .map_err(|e| UltraHonkError::Spawn {
                reason: format!("cannot run `{}`: {e}", self.binary.display()),
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        parse_version(if stdout.trim().is_empty() {
            &stderr
        } else {
            &stdout
        })
    }

    /// Checks that the installed version is supported by this adapter.
    ///
    /// Returns `Ok(())` when the major version is at least [`MIN_BB_MAJOR`],
    /// or [`UltraHonkError::UnsupportedBbVersion`] otherwise. The exact
    /// validated pairing lives in [`crate::backend::BACKEND_COMPAT`]; CI
    /// installs that pinned version, so this check is a floor against
    /// pre-2026 CLI generations rather than an exact-match gate.
    pub fn check_version(&self) -> Result<(), UltraHonkError> {
        let version = self.version()?;
        if version.major < MIN_BB_MAJOR {
            return Err(UltraHonkError::UnsupportedBbVersion {
                found: version.raw,
                supported: MIN_BB_MAJOR,
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

/// Parses `bb --version` output like `6.0.0-nightly.20260903`.
fn parse_version(stdout: &str) -> Result<BbVersion, UltraHonkError> {
    let first = stdout
        .lines()
        .next()
        .ok_or_else(|| UltraHonkError::VersionParse("empty output".to_owned()))?;
    let raw = first
        .split_once('=')
        .map(|(_, v)| v.trim())
        .unwrap_or(first.trim());
    let raw = raw.trim();
    // The major is the leading dotted number ("6" in "6.0.0-nightly...").
    let digits: String = raw
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let major: u32 = digits
        .split('.')
        .next()
        .and_then(|m| m.parse().ok())
        .ok_or_else(|| UltraHonkError::VersionParse(raw.to_owned()))?;
    Ok(BbVersion {
        raw: raw.to_owned(),
        major,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nightly_version_output() {
        let parsed = parse_version("6.0.0-nightly.20260903\n").unwrap();
        assert_eq!(parsed.raw, "6.0.0-nightly.20260903");
        assert_eq!(parsed.major, 6);
    }

    #[test]
    fn parses_bare_legacy_version_output() {
        let parsed = parse_version("0.87.0\n").unwrap();
        assert_eq!(parsed.raw, "0.87.0");
        assert_eq!(parsed.major, 0);
    }

    #[test]
    fn rejects_garbage_version_output() {
        assert!(parse_version("").is_err());
        assert!(parse_version("bb: command not found\n").is_err());
    }

    #[test]
    fn version_gate_floors_modern_generation() {
        let modern = BbVersion {
            raw: "6.0.0-nightly.20260903".to_owned(),
            major: 6,
        };
        assert!(modern.major >= MIN_BB_MAJOR);
        let legacy = BbVersion {
            raw: "0.87.0".to_owned(),
            major: 0,
        };
        assert!(legacy.major < MIN_BB_MAJOR);
    }
}
