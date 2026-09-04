//! Parsing `nargo info` output into structured circuit metrics.
//!
//! `nargo info` prints a table of ACIR/Brillig opcode counts per package and
//! function. The gadget structure of the repo relies on these per-circuit
//! measurements, so the parser lives here and is covered by golden tests
//! against captured real output.

use crate::errors::NoirError;

/// Circuit metrics for one `(package, function)` row of `nargo info`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitMetrics {
    /// Package name.
    pub package: String,
    /// Function name (usually `main`).
    pub function: String,
    /// ACIR opcode count.
    pub acir_opcodes: u64,
    /// Brillig opcode count.
    pub brillig_opcodes: u64,
}

/// The full parsed result of `nargo info`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InfoOutput {
    /// One metrics row per circuit in the output.
    pub circuits: Vec<CircuitMetrics>,
}

impl InfoOutput {
    /// Finds metrics for `package`, if present.
    pub fn for_package(&self, package: &str) -> Option<&CircuitMetrics> {
        self.circuits.iter().find(|c| c.package == package)
    }
}

/// Parses `nargo info` table output.
///
/// The parser is tolerant: it scans lines for rows that look like
/// `| package | function | N | M |` and ignores everything else, so minor
/// column changes by the tool do not break parsing.
pub fn parse_info_output(stdout: &str) -> Result<InfoOutput, NoirError> {
    let mut circuits = Vec::new();
    for line in stdout.lines() {
        let cells: Vec<&str> = line
            .trim()
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        if cells.len() < 4 {
            continue;
        }
        let (package, function) = (cells[0], cells[1]);
        if package.is_empty() || function.is_empty() {
            continue;
        }
        let (Ok(acir), Ok(brillig)) = (cells[2].parse::<u64>(), cells[3].parse::<u64>()) else {
            continue;
        };
        // Skip header-like rows where the "package" cell is not a real name.
        if package == "Package" {
            continue;
        }
        circuits.push(CircuitMetrics {
            package: package.to_owned(),
            function: function.to_owned(),
            acir_opcodes: acir,
            brillig_opcodes: brillig,
        });
    }
    if circuits.is_empty() {
        return Err(NoirError::VersionParse(
            "nargo info produced no parseable circuit rows".to_owned(),
        ));
    }
    Ok(InfoOutput { circuits })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured from `nargo 1.0.0-beta.26 info` on a minimal circuit.
    const SAMPLE: &str = "\
+---------+----------+--------------+-----------------+
| Package | Function | ACIR Opcodes | Brillig Opcodes |
+=========+==========+==============+=================+
| demo    | main     | 1            | 0               |
+---------+----------+--------------+-----------------+
";

    #[test]
    fn parses_captured_info_table() {
        let parsed = parse_info_output(SAMPLE).unwrap();
        assert_eq!(parsed.circuits.len(), 1);
        let metrics = parsed.for_package("demo").unwrap();
        assert_eq!(metrics.function, "main");
        assert_eq!(metrics.acir_opcodes, 1);
        assert_eq!(metrics.brillig_opcodes, 0);
    }

    #[test]
    fn rejects_output_without_circuit_rows() {
        assert!(parse_info_output("no table here\n").is_err());
        assert!(parse_info_output("").is_err());
    }

    #[test]
    fn handles_multiple_packages() {
        let multi = SAMPLE.replace("| demo    | main", "| a       | main") + SAMPLE;
        let parsed = parse_info_output(&multi).unwrap();
        assert_eq!(parsed.circuits.len(), 2);
        assert_eq!(parsed.for_package("a").unwrap().acir_opcodes, 1);
    }
}
