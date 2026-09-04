//! The [`UltraHonkVerifier`]: a real [`Verifier`] backed by `bb`.
//!
//! Verification re-runs the actual Barretenberg verifier over the submitted
//! context, rebuilding the backend artifact files from the wire types:
//!
//! - the proof blob (32-byte field words, concatenated at generation) is
//!   re-split into the `proof.json` word list;
//! - the verification key is resolved **from the [`VkStore`] by id** — a
//!   proof never carries key material, and an id that resolves to nothing is
//!   a wrong-key rejection before any cryptography runs;
//! - the submitted public outputs are re-encoded as the `public_inputs.json`
//!   word list the backend checks the proof against.
//!
//! `bb verify` then decides: exit 0 is acceptance, anything else is a
//! rejected proof. Backend failure reasons are coarse — an opaque UltraHonk
//! proof cannot say *why* it failed — so crypto rejections map to
//! [`VerificationFailure::InvalidProof`]; the precise context rejections
//! (wrong circuit/version/key/artifact) are detected structurally before bb
//! runs, mirroring how the mock reports them.
//!
//! # State binding
//!
//! The state-bound circuits (merge/transfer/withdraw) take the two halves
//! of the ledger state root as public parameters and fold them into the
//! emitted nullifier, so the proof bytes themselves commit to the root.
//! Verification enforces binding at two layers, mirroring the mock:
//!
//! 1. **Structural** — the submitted [`crucible_interfaces::StateReference`]
//!    must agree with the `root_hi`/`root_lo` words the proof committed to
//!    (checked before bb runs, giving a precise `StateReferenceMismatch` /
//!    `MissingStateBinding` reason);
//! 2. **Cryptographic** — a submission that rewrites the root words to a
//!    different root fails `bb verify`, because the proof was cut for the
//!    original root.
//!
//! Register and deposit circuits remain unbound (no root params), so their
//! proofs carry no root words and no state check applies.

use crucible_interfaces::{
    BackendId, ProofBlob, VerificationFailure, VerificationOutcome, VerificationRequest, Verifier,
    VerifierError,
};
use serde_json::json;

use crate::errors::UltraHonkError;
use crate::exec::{SCHEME_ULTRA_HONK, VerifyOptions, verify};
use crate::store::VkStore;
use crate::toolchain::BbToolchain;
use crate::vk::VerificationKeyIdPolicy;
use crate::{PROOF_FORMAT_TAG, UltraHonkBackend};

/// Byte width of one proof field word.
const WORD_BYTES: usize = 32;

/// A real UltraHonk [`Verifier`] (Barretenberg `bb verify`).
#[derive(Debug, Clone)]
pub struct UltraHonkVerifier {
    store: VkStore,
}

impl UltraHonkVerifier {
    /// Creates a verifier that resolves verification keys from `store`.
    pub fn new(store: VkStore) -> UltraHonkVerifier {
        UltraHonkVerifier { store }
    }

    /// The verification-key store backing this verifier.
    pub fn vk_store(&self) -> &VkStore {
        &self.store
    }
}

impl Verifier for UltraHonkVerifier {
    fn backend(&self) -> BackendId {
        UltraHonkBackend::id()
    }

    fn verify(&self, request: &VerificationRequest) -> Result<VerificationOutcome, VerifierError> {
        // 1. Routing: this verifier only understands UltraHonk proofs.
        if request.proof.format.as_str() != PROOF_FORMAT_TAG
            || request.backend != self.backend()
        {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::BackendMismatch,
            ));
        }

        // 2. The verification-key id pins circuit, version, and artifact.
        let (id_circuit, id_version, id_artifact) =
            match VerificationKeyIdPolicy::parse(request.verification_key_id.as_str()) {
                Ok(parts) => parts,
                Err(_) => {
                    return Ok(VerificationOutcome::rejected(
                        VerificationFailure::WrongVerificationKey,
                    ));
                }
            };
        if id_circuit != request.circuit {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::CircuitMismatch,
            ));
        }
        if id_version != request.circuit_version {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::VersionMismatch,
            ));
        }
        if id_artifact != request.artifact_checksum {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::ArtifactChecksumMismatch,
            ));
        }

        // 3. Resolve the verification key; an id that the store cannot
        //    resolve means the proof claims a key this verifier has never
        //    seen — a wrong-key rejection before any cryptography.
        let vk = match self.store.get(&request.verification_key_id) {
            Ok(vk) => vk,
            Err(UltraHonkError::MissingFile { .. }) => {
                return Ok(VerificationOutcome::rejected(
                    VerificationFailure::WrongVerificationKey,
                ));
            }
            Err(e) => {
                return Err(VerifierError::Internal {
                    reason: format!("cannot load verification key: {e}"),
                });
            }
        };

        // 4. Structural integrity of the proof bytes: whole field words only.
        if request.proof.bytes.is_empty() || !request.proof.bytes.len().is_multiple_of(WORD_BYTES) {
            return Ok(VerificationOutcome::rejected(
                VerificationFailure::InvalidProof,
            ));
        }

        // 5. State binding (state-bound circuits). The provider names every
        //    public-input word from the pinned circuit surface, so a proof
        //    for a state-bound operation always carries `root_hi`/`root_lo`
        //    — the two halves of the ledger root the circuit committed to.
        //    Mirroring the mock, a submission whose state reference disagrees
        //    with those committed words is stale or replayed, rejected
        //    structurally before any cryptography runs. If the submitter also
        //    rewrote the words themselves, bb rejects the tamper below.
        if let (Some(root_hi), Some(root_lo)) = (
            request.public_outputs.get("root_hi"),
            request.public_outputs.get("root_lo"),
        ) {
            let state = match &request.state_reference {
                Some(state) => state,
                None => {
                    return Ok(VerificationOutcome::rejected(
                        VerificationFailure::MissingStateBinding,
                    ));
                }
            };
            let (hi, lo) = state.root_halves();
            if root_hi != &hi || root_lo != &lo {
                return Ok(VerificationOutcome::rejected(
                    VerificationFailure::StateReferenceMismatch,
                ));
            }
        }

        // 6. Rebuild the backend artifact files in a scratch dir and let bb
        //    decide. The submitted public outputs are the words bb checks
        //    the proof against, so a changed output fails here, cryptographically.
        let bb = match BbToolchain::locate() {
            Ok(toolchain) => toolchain,
            Err(_) => {
                return Err(VerifierError::VerificationUnavailable {
                    backend: BackendId::ULTRAHONK.to_owned(),
                    reason: "bb binary not found on PATH (see scripts/check-bb.sh)".to_owned(),
                });
            }
        };
        bb.check_version()
            .map_err(|e| VerifierError::VerificationUnavailable {
                backend: BackendId::ULTRAHONK.to_owned(),
                reason: e.to_string(),
            })?;

        let dir = tempfile::tempdir().map_err(|e| VerifierError::Internal {
            reason: format!("cannot create verification scratch dir: {e}"),
        })?;
        let proof_path = dir.path().join("proof.json");
        let vk_path = dir.path().join("vk.json");
        let public_inputs_path = dir.path().join("public_inputs.json");

        write_proof_json(&proof_path, &request.proof, &vk)?;
        write_vk_json(&vk_path, &vk)?;
        write_public_inputs_json(&public_inputs_path, request)?;

        let outcome = verify(
            &bb,
            &VerifyOptions {
                proof: &proof_path,
                vk: &vk_path,
                public_inputs: &public_inputs_path,
            },
        )
        .map_err(|e| VerifierError::VerificationUnavailable {
            backend: BackendId::ULTRAHONK.to_owned(),
            reason: format!("bb verify failed to run: {e}"),
        })?;

        if outcome.verified {
            Ok(VerificationOutcome::verified())
        } else {
            Ok(VerificationOutcome::rejected(
                VerificationFailure::InvalidProof,
            ))
        }
    }
}

/// Writes `proof.json` from the proof blob's field words, embedding the
/// digest of the resolved verification key.
fn write_proof_json(path: &std::path::Path, proof: &ProofBlob, vk: &crate::exec::VkDocument) -> Result<(), VerifierError> {
    let words: Vec<String> = proof
        .bytes
        .chunks(WORD_BYTES)
        .map(|chunk| format!("0x{}", hex::encode(chunk)))
        .collect();
    let doc = json!({
        "proof": words,
        "vk_hash": vk.hash,
        "bb_version": vk.bb_version,
        "scheme": vk.scheme,
    });
    std::fs::write(path, serde_json::to_string(&doc).map_err(|e| VerifierError::Internal {
        reason: format!("cannot serialize proof document: {e}"),
    })?)
    .map_err(|e| VerifierError::Internal {
        reason: format!("cannot write proof document: {e}"),
    })
}

/// Writes `vk.json` verbatim from the resolved verification key.
fn write_vk_json(path: &std::path::Path, vk: &crate::exec::VkDocument) -> Result<(), VerifierError> {
    let doc = json!({
        "vk": vk.vk,
        "hash": vk.hash,
        "bb_version": vk.bb_version,
        "scheme": vk.scheme,
    });
    std::fs::write(path, serde_json::to_string(&doc).map_err(|e| VerifierError::Internal {
        reason: format!("cannot serialize verification key: {e}"),
    })?)
    .map_err(|e| VerifierError::Internal {
        reason: format!("cannot write verification key: {e}"),
    })
}

/// Writes `public_inputs.json` from the submitted public outputs, in bag
/// order (the provider inserts them in circuit surface order).
fn write_public_inputs_json(
    path: &std::path::Path,
    request: &VerificationRequest,
) -> Result<(), VerifierError> {
    let words: Vec<String> = request
        .public_outputs
        .iter()
        .map(|(_, value)| {
            // Field values are canonical (no 0x, no leading zeros); bb words
            // are 32-byte zero-padded.
            format!("0x{:0>64}", value.as_hex())
        })
        .collect();
    let doc = json!({
        "public_inputs": words,
        "bb_version": crate::TESTED_BB_VERSION,
        "scheme": SCHEME_ULTRA_HONK,
    });
    std::fs::write(path, serde_json::to_string(&doc).map_err(|e| VerifierError::Internal {
        reason: format!("cannot serialize public inputs: {e}"),
    })?)
    .map_err(|e| VerifierError::Internal {
        reason: format!("cannot write public inputs: {e}"),
    })
}
