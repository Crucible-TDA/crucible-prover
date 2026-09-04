//! Redacted provenance transcript.
//!
//! Every prove/verify run in Crucible should be auditable: *what* was
//! requested, *which* circuit and backend answered, *what* the outcome was.
//! But a transcript must never carry private witness material. This module
//! writes JSON-lines transcripts whose entries are constructed from the
//! redacted view of a request — names and counts, never values — so the log
//! is safe for CI output and bug reports.

use std::io::Write;
use std::path::Path;

use crucible_interfaces::{Operation, ProofResponse, VerificationOutcome};

use crate::errors::CoreError;

/// One redacted line of a provenance transcript.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TranscriptEntry {
    /// The operation that was requested.
    pub operation: Operation,
    /// The circuit that served the request.
    pub circuit: String,
    /// The backend that served the request.
    pub backend: String,
    /// The request id, for cross-referencing.
    pub request_id: String,
    /// Number of private witness values (names never logged).
    pub private_witness_count: usize,
    /// Verification key the proof is valid under.
    pub verification_key_id: String,
    /// Artifact checksum the proof claims to come from.
    pub artifact_checksum: String,
    /// Whether the proof verified in the local round-trip.
    pub verified: bool,
}

impl TranscriptEntry {
    /// Builds a redacted entry from a request and its outcome.
    ///
    /// Private values are never included: only the request's witness *count*
    /// and the public provenance of the response (verification key id,
    /// artifact checksum) are recorded.
    pub fn from_request_response(
        request: &crucible_interfaces::ProofRequest,
        response: &ProofResponse,
        outcome: &VerificationOutcome,
    ) -> TranscriptEntry {
        TranscriptEntry {
            operation: request.operation,
            circuit: request.circuit.to_string(),
            backend: request.backend.to_string(),
            request_id: request.request_id.to_string(),
            private_witness_count: request.witness.len(),
            verification_key_id: response.verification_key_id.to_string(),
            artifact_checksum: response.artifact_checksum.to_string(),
            verified: outcome.verified,
        }
    }
}

/// Appends a redacted transcript entry to a JSON-lines file, creating it if
/// needed. Never contains witness values.
pub fn append_transcript(path: &Path, entry: &TranscriptEntry) -> Result<(), CoreError> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| {
            CoreError::Internal(format!("cannot open transcript {}: {e}", path.display()))
        })?;
    let line = serde_json::to_string(entry)
        .map_err(|e| CoreError::Internal(format!("cannot encode transcript: {e}")))?;
    writeln!(file, "{line}")
        .map_err(|e| CoreError::Internal(format!("cannot write transcript: {e}")))
}

/// A buffered transcript writer for batch runs.
#[derive(Debug)]
pub struct TranscriptWriter {
    inner: Option<std::io::BufWriter<std::fs::File>>,
}

impl TranscriptWriter {
    /// Creates a writer appending to `path`.
    pub fn append(path: &Path) -> Result<TranscriptWriter, CoreError> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| {
                CoreError::Internal(format!("cannot open transcript {}: {e}", path.display()))
            })?;
        Ok(TranscriptWriter {
            inner: Some(std::io::BufWriter::new(file)),
        })
    }

    /// A writer that discards everything (useful for library defaults).
    pub fn null() -> TranscriptWriter {
        TranscriptWriter { inner: None }
    }

    /// Writes one redacted entry.
    pub fn write(&mut self, entry: &TranscriptEntry) -> Result<(), CoreError> {
        let Some(writer) = self.inner.as_mut() else {
            return Ok(());
        };
        let line = serde_json::to_string(entry)
            .map_err(|e| CoreError::Internal(format!("cannot encode transcript: {e}")))?;
        writeln!(writer, "{line}")
            .map_err(|e| CoreError::Internal(format!("cannot write transcript: {e}")))
    }

    /// Flushes pending entries.
    pub fn flush(&mut self) -> Result<(), CoreError> {
        if let Some(writer) = self.inner.as_mut() {
            writer
                .flush()
                .map_err(|e| CoreError::Internal(format!("cannot flush transcript: {e}")))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crucible_interfaces::{ProofProvider, Verifier};
    use crucible_mock::{MockProver, MockVerifier, fixtures};

    #[test]
    fn transcript_never_contains_secret_values() {
        let request = fixtures::transfer_request();
        let response = MockProver::new().generate(&request).unwrap();
        let outcome = MockVerifier::new()
            .verify(&crucible_interfaces::VerificationRequest::from_response(
                &response,
            ))
            .unwrap();
        let entry = TranscriptEntry::from_request_response(&request, &response, &outcome);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");
        append_transcript(&path, &entry).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();

        // Private values from the fixtures must not appear anywhere.
        for secret in ["deadbeefcafe", "112233445566778899aabbccddeeff00"] {
            assert!(!content.contains(secret), "transcript leaked {secret}");
        }
        assert!(content.contains("\"verified\":true"));
        assert!(content.contains("\"circuit\":\"transfer\""));
        assert!(content.contains("\"private_witness_count\":3"));
    }

    #[test]
    fn writer_appends_and_flushes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("batch.jsonl");
        let mut writer = TranscriptWriter::append(&path).unwrap();
        let request = fixtures::transfer_request();
        let response = MockProver::new().generate(&request).unwrap();
        let outcome = MockVerifier::new()
            .verify(&crucible_interfaces::VerificationRequest::from_response(
                &response,
            ))
            .unwrap();
        let entry = TranscriptEntry::from_request_response(&request, &response, &outcome);
        writer.write(&entry).unwrap();
        writer.flush().unwrap();
        drop(writer);

        // Appending again must not truncate.
        let mut writer = TranscriptWriter::append(&path).unwrap();
        writer.write(&entry).unwrap();
        writer.flush().unwrap();
        drop(writer);
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 2);
    }
}
