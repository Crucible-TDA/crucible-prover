//! Aggregated results of running one proof through several verifiers.

use crucible_interfaces::{VerificationFailure, VerificationOutcome};

/// The outcome of one verifier in a multi-verifier run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierResult {
    /// Human label of the verifier that ran (e.g. `local` or `soroban`).
    pub label: String,
    /// Outcome it returned.
    pub outcome: VerificationOutcome,
}

impl VerifierResult {
    /// Whether this verifier accepted the proof.
    pub fn verified(&self) -> bool {
        self.outcome.verified
    }
}

/// The aggregate answer of running one proof through several verifiers.
///
/// The central question this type answers: *do the verifiers agree?*
/// Crucible must not assume local verification and on-chain verification are
/// equivalent; an explicit report that surfaces disagreement is the first
/// step to catching backend/encoding drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    /// One result per verifier that ran.
    pub results: Vec<VerifierResult>,
}

impl VerificationReport {
    /// Whether every verifier accepted the proof.
    pub fn all_verified(&self) -> bool {
        !self.results.is_empty() && self.results.iter().all(|r| r.verified())
    }

    /// Whether every verifier rejected the proof for the same reason.
    pub fn all_rejected_with(&self, failure: VerificationFailure) -> bool {
        !self.results.is_empty()
            && self
                .results
                .iter()
                .all(|r| r.outcome.rejected_with(failure))
    }

    /// Whether at least one verifier accepted the proof.
    pub fn any_verified(&self) -> bool {
        self.results.iter().any(|r| r.verified())
    }

    /// Verifiers that accepted the proof.
    pub fn verified_by(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter(|r| r.verified())
            .map(|r| r.label.as_str())
            .collect()
    }

    /// Whether verifiers disagree: at least one accepted and at least one
    /// rejected.
    pub fn disagrees(&self) -> bool {
        self.results.len() > 1 && self.any_verified() && !self.all_verified()
    }

    /// Labels of verifiers that rejected the proof, with their reasons.
    pub fn rejections(&self) -> Vec<(&str, Option<VerificationFailure>)> {
        self.results
            .iter()
            .filter(|r| !r.verified())
            .map(|r| (r.label.as_str(), r.outcome.failure))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accept(label: &str) -> VerifierResult {
        VerifierResult {
            label: label.to_owned(),
            outcome: VerificationOutcome::verified(),
        }
    }

    fn reject(label: &str, failure: VerificationFailure) -> VerifierResult {
        VerifierResult {
            label: label.to_owned(),
            outcome: VerificationOutcome::rejected(failure),
        }
    }

    #[test]
    fn unanimous_acceptance_is_not_a_disagreement() {
        let report = VerificationReport {
            results: vec![accept("local"), accept("soroban")],
        };
        assert!(report.all_verified());
        assert!(!report.disagrees());
        assert_eq!(report.verified_by(), vec!["local", "soroban"]);
    }

    #[test]
    fn split_verdict_is_a_disagreement() {
        let report = VerificationReport {
            results: vec![
                accept("local"),
                reject("soroban", VerificationFailure::InvalidProof),
            ],
        };
        assert!(!report.all_verified());
        assert!(report.any_verified());
        assert!(report.disagrees());
        assert_eq!(report.verified_by(), vec!["local"]);
        assert_eq!(
            report.rejections(),
            vec![("soroban", Some(VerificationFailure::InvalidProof))]
        );
    }

    #[test]
    fn unanimous_rejection_with_reason() {
        let report = VerificationReport {
            results: vec![
                reject("local", VerificationFailure::WrongVerificationKey),
                reject("soroban", VerificationFailure::WrongVerificationKey),
            ],
        };
        assert!(report.all_rejected_with(VerificationFailure::WrongVerificationKey));
        assert!(!report.disagrees());
    }

    #[test]
    fn empty_report_is_not_all_verified() {
        let report = VerificationReport { results: vec![] };
        assert!(!report.all_verified());
        assert!(!report.disagrees());
    }
}
