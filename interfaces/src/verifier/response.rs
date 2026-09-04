use std::fmt;

/// Structured reason a verification failed.
///
/// Security tests assert on these reasons so a regression can distinguish "a
/// tampered proof was accepted" from "the wrong test failed".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerificationFailure {
    /// The proof bytes did not verify under the given key.
    InvalidProof,
    /// The proof verified, but the submitted public outputs differ from what
    /// the proof commits to.
    PublicOutputMismatch,
    /// The proof verified, but it is bound to a different state reference
    /// (stale state or replay attempt).
    StateReferenceMismatch,
    /// The proof was produced under a different verification key.
    WrongVerificationKey,
    /// The proof is for a different circuit.
    CircuitMismatch,
    /// The proof is for a different circuit version.
    VersionMismatch,
    /// The artifact checksum does not match the artifact that produced the
    /// proof (tampered or replaced artifact).
    ArtifactChecksumMismatch,
    /// The proof's format/backend does not match the verifier.
    BackendMismatch,
    /// The request carried no state reference but the proof requires one, or
    /// the proof is unbound where binding is mandatory.
    MissingStateBinding,
}

impl fmt::Display for VerificationFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            VerificationFailure::InvalidProof => "proof bytes did not verify",
            VerificationFailure::PublicOutputMismatch => "public outputs do not match the proof",
            VerificationFailure::StateReferenceMismatch => {
                "state reference does not match the proof"
            }
            VerificationFailure::WrongVerificationKey => {
                "proof was not produced under this verification key"
            }
            VerificationFailure::CircuitMismatch => "proof is for a different circuit",
            VerificationFailure::VersionMismatch => "proof is for a different circuit version",
            VerificationFailure::ArtifactChecksumMismatch => {
                "artifact checksum does not match the proof"
            }
            VerificationFailure::BackendMismatch => "proof format does not match this verifier",
            VerificationFailure::MissingStateBinding => {
                "proof requires a state binding that was not provided"
            }
        };
        f.write_str(msg)
    }
}

/// The outcome of running one verification.
///
/// `verified == false` is a *valid* outcome, not an error: rejection of
/// tampered, stale, or misattributed proofs is the expected behavior and
/// carries a [`VerificationFailure`] reason so callers can react precisely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationOutcome {
    /// Whether the proof is valid for the submitted context.
    pub verified: bool,
    /// The reason, when not verified.
    pub failure: Option<VerificationFailure>,
}

impl VerificationOutcome {
    /// A positive outcome.
    pub fn verified() -> VerificationOutcome {
        VerificationOutcome {
            verified: true,
            failure: None,
        }
    }

    /// A negative outcome with a reason.
    pub fn rejected(failure: VerificationFailure) -> VerificationOutcome {
        VerificationOutcome {
            verified: false,
            failure: Some(failure),
        }
    }

    /// Whether this outcome is a rejection with the given reason.
    pub fn rejected_with(&self, failure: VerificationFailure) -> bool {
        !self.verified && self.failure == Some(failure)
    }
}

impl fmt::Display for VerificationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.failure {
            Some(reason) => write!(f, "rejected: {reason}"),
            None => write!(f, "verified"),
        }
    }
}
