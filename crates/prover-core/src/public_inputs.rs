//! Public-input binding checks.
//!
//! A proof is only as good as the public context it is bound to. These
//! helpers compare *expected* public outputs (what the caller believes the
//! circuit should output) against *produced* outputs (what the response
//! actually commits to) so mismatches are detected structurally, before or
//! after verification.

use crucible_interfaces::{OutputBag, PublicInputBag};

/// Names that differ between `expected` and `produced`, in insertion order of
/// the first bag. Names present in one bag but not the other are reported
/// with their side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputMismatch {
    /// Names whose values differ between the two bags.
    pub differing: Vec<String>,
    /// Names present only in the expected bag.
    pub missing_from_produced: Vec<String>,
    /// Names present only in the produced bag.
    pub unexpected: Vec<String>,
}

impl OutputMismatch {
    /// Whether the two bags are exactly equal (empty mismatch).
    pub fn is_empty(&self) -> bool {
        self.differing.is_empty()
            && self.missing_from_produced.is_empty()
            && self.unexpected.is_empty()
    }
}

/// Compares two public output bags structurally.
pub fn compare_outputs(expected: &PublicInputBag, produced: &OutputBag) -> OutputMismatch {
    let expected_names: Vec<&str> = expected.names().collect();
    let produced_names: Vec<&str> = produced.names().collect();

    let mut differing = Vec::new();
    let mut missing_from_produced = Vec::new();
    let mut unexpected = Vec::new();

    for name in &expected_names {
        match (expected.get(name), produced.get(name)) {
            (Some(a), Some(b)) if a != b => differing.push((*name).to_owned()),
            (Some(_), None) => missing_from_produced.push((*name).to_owned()),
            _ => {}
        }
    }
    for name in &produced_names {
        if expected.get(name).is_none() {
            unexpected.push((*name).to_owned());
        }
    }

    OutputMismatch {
        differing,
        missing_from_produced,
        unexpected,
    }
}

/// Whether `produced` binds exactly the outputs `expected` declares.
pub fn outputs_match(expected: &PublicInputBag, produced: &OutputBag) -> bool {
    compare_outputs(expected, produced).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::FieldValue;

    fn bag(pairs: &[(&str, &str)]) -> PublicInputBag {
        let mut bag = PublicInputBag::new();
        for (name, hex) in pairs {
            bag.insert(*name, FieldValue::from_hex(hex).unwrap())
                .unwrap();
        }
        bag
    }

    #[test]
    fn identical_bags_match() {
        let a = bag(&[("sender", "aa"), ("amount", "07")]);
        let b = bag(&[("sender", "aa"), ("amount", "07")]);
        assert!(outputs_match(&a, &b));
        assert!(compare_outputs(&a, &b).is_empty());
    }

    #[test]
    fn differing_values_are_reported() {
        let expected = bag(&[("sender", "aa"), ("amount", "07")]);
        let produced = bag(&[("sender", "bb"), ("amount", "07")]);
        let mismatch = compare_outputs(&expected, &produced);
        assert_eq!(mismatch.differing, vec!["sender"]);
        assert!(!outputs_match(&expected, &produced));
    }

    #[test]
    fn side_mismatches_are_reported() {
        let expected = bag(&[("sender", "aa"), ("amount", "07")]);
        let produced = bag(&[("sender", "aa")]);
        let mismatch = compare_outputs(&expected, &produced);
        assert_eq!(mismatch.missing_from_produced, vec!["amount"]);

        let produced = bag(&[("sender", "aa"), ("extra", "01")]);
        let mismatch = compare_outputs(&expected, &produced);
        assert_eq!(mismatch.unexpected, vec!["extra"]);
    }
}
