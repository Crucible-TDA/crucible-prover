//! `crucible-prover benchmark` — measure the proving pipeline phases.
//!
//! Runs a valid vector's request through the chosen backend repeatedly and
//! reports per-phase timings (prove, verify, envelope serialization) plus
//! proof/envelope sizes. The mock backend measures orchestration overhead
//! with zero cryptography; the ultrahonk backend measures real UltraHonk
//! proving through `bb` (requires nargo + bb on PATH and pinned bytecode).
//! Either way the numbers are honest about what they cover — the mock
//! backend always prints a warning that it is not cryptographically secure.
//!
//! This is the CLI half of the plan's benchmark surface; the Cargo bench
//! harness in `benches/` covers the same phases in-process.

use std::path::PathBuf;
use std::time::Instant;

use clap::Args;
use crucible_interfaces::prover::Prover;
use crucible_interfaces::{BackendId, VerificationRequest, Verifier};
use crucible_proof_types::{ProofEnvelope, common::ENVELOPE_V1};
use crucible_prover_core::ProverService;
use crucible_vectors::TestVector;

use crate::commands::circuits::OPERATIONS;
use crate::paths;

#[derive(Debug, Args)]
pub struct BenchmarkArgs {
    /// Operation circuit to benchmark.
    pub op: String,
    /// Test-vector JSON file describing the witness; defaults to the
    /// operation's committed valid vector (`<catalog>/<op>/valid/…`).
    #[arg(long, value_name = "PATH")]
    pub vector: Option<PathBuf>,
    /// Backend to prove with: `mock` (TEST ONLY, fast) or `ultrahonk`
    /// (real proofs; requires nargo + bb on PATH and compiled bytecode).
    #[arg(long, default_value = "mock", value_name = "BACKEND")]
    pub backend: String,
    /// Number of prove→verify iterations to time.
    #[arg(long, default_value_t = 3, value_name = "N")]
    pub iterations: u64,
    /// Verification-key store directory (ultrahonk backend; defaults to
    /// `<repo>/artifacts/verification-keys`).
    #[arg(long, value_name = "DIR")]
    pub vk_store: Option<PathBuf>,
}

/// Per-phase timing accumulator.
struct Timing {
    count: u64,
    total_ms: f64,
    min_ms: f64,
    max_ms: f64,
}

impl Timing {
    fn new() -> Timing {
        Timing {
            count: 0,
            total_ms: 0.0,
            min_ms: f64::INFINITY,
            max_ms: 0.0,
        }
    }

    fn record(&mut self, elapsed: std::time::Duration) {
        let ms = elapsed.as_secs_f64() * 1000.0;
        self.count += 1;
        self.total_ms += ms;
        self.min_ms = self.min_ms.min(ms);
        self.max_ms = self.max_ms.max(ms);
    }

    fn avg_ms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_ms / self.count as f64
        }
    }
}

pub fn run(args: BenchmarkArgs, circuits: &std::path::Path) -> Result<(), String> {
    if !OPERATIONS.contains(&args.op.as_str()) {
        return Err(format!(
            "unknown circuit `{}` (expected one of {})",
            args.op,
            OPERATIONS.join(", ")
        ));
    }
    if args.iterations == 0 {
        return Err("--iterations must be at least 1".to_owned());
    }

    let vector = match &args.vector {
        Some(path) => {
            let vector = TestVector::load(path)
                .map_err(|e| format!("cannot load vector `{}`: {e}", path.display()))?;
            if vector.operation.as_str() != args.op {
                return Err(format!(
                    "operation `{}` does not match vector `{}` (operation {})",
                    args.op, vector.id, vector.operation
                ));
            }
            vector
        }
        None => {
            let path = paths::default_catalog_root()
                .join(&args.op)
                .join("valid")
                .join(format!("{}-valid-001.json", args.op));
            TestVector::load(&path).map_err(|e| {
                format!(
                    "no default valid vector for `{}` at `{}`: {e} (pass --vector)",
                    args.op,
                    path.display()
                )
            })?
        }
    };
    if !vector.expect_verification {
        return Err(format!(
            "vector `{}` is a rejecting vector (expect_verification=false); \
             benchmarking is only meaningful for witnesses expected to verify",
            vector.id
        ));
    }

    let request = vector.to_request_for(&args.backend);
    request
        .validate()
        .map_err(|e| format!("request is structurally invalid: {e}"))?;

    let mut service = ProverService::new(concat!("crucible-prover/", env!("CARGO_PKG_VERSION")));
    let verifier: Box<dyn Verifier> = match args.backend.as_str() {
        BackendId::MOCK => {
            let verifier = crucible_mock::MockVerifier::new();
            service.register_provider(Box::new(crucible_mock::MockProver::new()));
            Box::new(verifier)
        }
        BackendId::ULTRAHONK => {
            if !crucible_noir::NoirToolchain::is_available()
                || !crucible_ultrahonk::BbToolchain::is_available()
            {
                return Err("ultrahonk benchmarking requires nargo and bb on PATH (see \
                     scripts/check-bb.sh); use --backend mock for orchestration overhead"
                    .to_owned());
            }
            let store = crucible_ultrahonk::VkStore::new(vk_store_or_default(&args.vk_store)?);
            let config = crucible_ultrahonk::UltraHonkConfig::new(circuits.to_path_buf(), store);
            let provider = crucible_ultrahonk::UltraHonkProvider::new(config);
            let verifier = crucible_ultrahonk::UltraHonkVerifier::new(provider.vk_store().clone());
            service.register_provider(Box::new(provider));
            Box::new(verifier)
        }
        other => {
            return Err(format!(
                "unknown backend `{other}` (expected `{}` or `{}`)",
                BackendId::MOCK,
                BackendId::ULTRAHONK
            ));
        }
    };

    let mut prove = Timing::new();
    let mut verify = Timing::new();
    let mut serialize = Timing::new();
    let mut response = None;
    let mut json_len = 0usize;

    for _ in 0..args.iterations {
        let t = Instant::now();
        let proved = service
            .prove(&request)
            .map_err(|e| format!("proving failed on iteration: {e}"))?;
        prove.record(t.elapsed());

        let t = Instant::now();
        let outcome = verifier
            .verify(&VerificationRequest::from_response(&proved))
            .map_err(|e| format!("verifier failed on iteration: {e}"))?;
        verify.record(t.elapsed());
        if !outcome.verified {
            return Err(format!(
                "proof failed its own verification on iteration: {}",
                outcome
                    .failure
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "unknown".to_owned())
            ));
        }

        let envelope = ProofEnvelope::from_response(
            &proved,
            vector.operation,
            concat!("crucible-prover/", env!("CARGO_PKG_VERSION")),
        );
        let t = Instant::now();
        json_len = envelope
            .to_json()
            .map_err(|e| format!("serializing the proof envelope failed: {e}"))?
            .len();
        serialize.record(t.elapsed());
        response = Some(proved);
    }

    let proved = response.expect("at least one iteration ran");
    println!(
        "benchmark   {} (backend {}) — {} iteration(s)",
        vector.id, args.backend, args.iterations
    );
    println!(
        "backend     {} ({} bytes, format {})",
        args.backend,
        proved.proof.bytes.len(),
        proved.proof.format
    );
    println!("envelope    {json_len} bytes json (format v{ENVELOPE_V1})");
    println!("public      {} word(s)", proved.public_outputs.len());
    println!(
        "state       {}",
        proved
            .state_reference
            .as_ref()
            .map(|s| format!("root {} seq {}", s.root, s.sequence))
            .unwrap_or_else(|| "unbound".to_owned())
    );
    println!(
        "prove       avg {:.3} ms  (min {:.3}, max {:.3})",
        prove.avg_ms(),
        prove.min_ms,
        prove.max_ms
    );
    println!(
        "verify      avg {:.3} ms  (min {:.3}, max {:.3})",
        verify.avg_ms(),
        verify.min_ms,
        verify.max_ms
    );
    println!(
        "serialize   avg {:.3} ms  (min {:.3}, max {:.3})",
        serialize.avg_ms(),
        serialize.min_ms,
        serialize.max_ms
    );
    if args.backend == BackendId::MOCK {
        println!("note        mock backend is TEST ONLY and not cryptographically secure");
    }
    Ok(())
}

/// Resolves the verification-key store directory, creating it on demand.
fn vk_store_or_default(explicit: &Option<PathBuf>) -> Result<PathBuf, String> {
    let dir = explicit.clone().unwrap_or_else(paths::default_vk_store);
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "cannot create verification-key store `{}`: {e}",
            dir.display()
        )
    })?;
    Ok(dir)
}
