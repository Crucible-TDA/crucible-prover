//! The [`VerificationService`] and verifier registration.

use crucible_interfaces::{BackendId, VerificationOutcome, VerificationRequest, Verifier};

use crate::errors::VerifierServiceError;
use crate::report::{VerificationReport, VerifierResult};

/// One registered verifier plus the human label it reports under.
pub struct VerifierRegistration {
    /// Label used in reports (e.g. `local`, `soroban`).
    pub label: String,
    /// The verifier implementation.
    pub verifier: Box<dyn Verifier>,
}

impl VerifierRegistration {
    /// Creates a registration.
    pub fn new(label: impl Into<String>, verifier: Box<dyn Verifier>) -> VerifierRegistration {
        VerifierRegistration {
            label: label.into(),
            verifier,
        }
    }
}

/// Routes verification requests to the registered verifier(s) for a backend.
///
/// A service is keyed by [`BackendId`]: every registered verifier must agree
/// on which backend it serves (the [`Verifier::backend`] identity), and a
/// request for backend `X` only ever runs verifiers registered for `X`. This
/// keeps a mock proof from accidentally being dispatched to an UltraHonk
/// verifier, and vice versa.
#[derive(Default)]
pub struct VerificationService {
    by_backend: Vec<(BackendId, VerifierRegistration)>,
}

impl VerificationService {
    /// Creates an empty service.
    pub fn new() -> VerificationService {
        VerificationService {
            by_backend: Vec::new(),
        }
    }

    /// Registers a verifier. Multiple verifiers may serve the same backend
    /// (e.g. a local UltraHonk verifier and a Soroban verifier), which is
    /// what makes cross-verifier agreement testing possible.
    pub fn register(
        &mut self,
        registration: VerifierRegistration,
    ) -> Result<(), VerifierServiceError> {
        let backend = registration.verifier.backend();
        self.by_backend.push((backend, registration));
        Ok(())
    }

    /// Convenience registration from label + verifier.
    pub fn register_verifier(
        &mut self,
        label: impl Into<String>,
        verifier: Box<dyn Verifier>,
    ) -> Result<(), VerifierServiceError> {
        self.register(VerifierRegistration::new(label, verifier))
    }

    /// Returns the labels of verifiers registered for `backend`.
    pub fn verifiers_for(&self, backend: &BackendId) -> Vec<&str> {
        self.by_backend
            .iter()
            .filter(|(b, _)| b == backend)
            .map(|(_, reg)| reg.label.as_str())
            .collect()
    }

    /// Runs `request` through every verifier registered for its backend and
    /// aggregates the results.
    ///
    /// Returns an error only if *no* verifier is registered for the request's
    /// backend, or a registered verifier fails to run. A rejected proof is a
    /// normal outcome, surfaced inside the [`VerificationReport`].
    pub fn verify(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationReport, VerifierServiceError> {
        let mut results = Vec::new();
        let mut ran_any = false;
        for (backend, registration) in &self.by_backend {
            if backend != &request.backend {
                continue;
            }
            ran_any = true;
            let outcome = registration.verifier.verify(request).map_err(|e| {
                VerifierServiceError::VerifierFailed {
                    label: registration.label.clone(),
                    reason: e.to_string(),
                }
            })?;
            results.push(VerifierResult {
                label: registration.label.clone(),
                outcome,
            });
        }
        if !ran_any {
            return Err(VerifierServiceError::UnknownBackend {
                backend: request.backend.to_string(),
            });
        }
        Ok(VerificationReport { results })
    }

    /// Runs `request` and returns `true` only if *every* registered verifier
    /// for its backend accepted the proof.
    pub fn verify_all(
        &self,
        request: &VerificationRequest,
    ) -> Result<VerificationOutcome, VerifierServiceError> {
        let report = self.verify(request)?;
        if report.all_verified() {
            Ok(VerificationOutcome::verified())
        } else {
            Ok(VerificationOutcome::rejected(
                report
                    .rejections()
                    .first()
                    .and_then(|(_, f)| *f)
                    .unwrap_or(crucible_interfaces::VerificationFailure::InvalidProof),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::{ProofProvider, VerificationFailure};
    use crucible_mock::{MockProver, MockVerifier, fixtures};

    fn mock_request() -> VerificationRequest {
        let request = fixtures::transfer_request();
        let response = MockProver::new().generate(&request).unwrap();
        VerificationRequest::from_response(&response)
    }

    fn service_with_two_verifiers() -> VerificationService {
        let mut service = VerificationService::new();
        service
            .register_verifier("local", Box::new(MockVerifier::new()))
            .unwrap();
        service
            .register_verifier("local-copy", Box::new(MockVerifier::new()))
            .unwrap();
        service
    }

    #[test]
    fn valid_proof_verifies_across_all_registered_verifiers() {
        let service = service_with_two_verifiers();
        let report = service.verify(&mock_request()).unwrap();
        assert_eq!(report.results.len(), 2);
        assert!(report.all_verified());
        assert!(!report.disagrees());
    }

    #[test]
    fn tampered_proof_is_rejected_everywhere() {
        let service = service_with_two_verifiers();
        let request = fixtures::transfer_request();
        let response = MockProver::new().generate(&request).unwrap();
        let mut tampered = VerificationRequest::from_response(&response);
        let last = tampered.proof.bytes.len() - 1;
        tampered.proof.bytes[last] ^= 0x01;
        let report = service.verify(&tampered).unwrap();
        assert!(report.all_rejected_with(VerificationFailure::InvalidProof));
        assert!(!report.disagrees());
    }

    #[test]
    fn disagreement_between_verifiers_is_detected() {
        let mut service = VerificationService::new();
        service
            .register_verifier("local", Box::new(MockVerifier::new()))
            .unwrap();
        // A verifier with a different key disagrees with the first.
        service
            .register_verifier("broken-copy", Box::new(MockVerifier::with_key("other")))
            .unwrap();
        let report = service.verify(&mock_request()).unwrap();
        assert!(report.disagrees(), "report must surface disagreement");
        assert_eq!(report.verified_by(), vec!["local"]);
        assert_eq!(report.rejections().len(), 1);
    }

    #[test]
    fn unknown_backend_is_an_error_not_a_rejection() {
        let service = VerificationService::new();
        let err = service.verify(&mock_request()).unwrap_err();
        assert!(matches!(err, VerifierServiceError::UnknownBackend { .. }));
    }

    #[test]
    fn verify_all_requires_unanimity() {
        let mut service = VerificationService::new();
        service
            .register_verifier("local", Box::new(MockVerifier::new()))
            .unwrap();
        service
            .register_verifier("broken-copy", Box::new(MockVerifier::with_key("other")))
            .unwrap();
        let outcome = service.verify_all(&mock_request()).unwrap();
        assert!(!outcome.verified);

        let service = service_with_two_verifiers();
        let outcome = service.verify_all(&mock_request()).unwrap();
        assert!(outcome.verified);
    }
}
