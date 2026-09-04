//! Shared helpers for the `crucible-benches` targets.
//!
//! Benches measure the parts of the pipeline that need no toolchain: mock
//! round trips (prove → verify → envelope) exercise the full orchestration
//! path with zero cryptography, witness assembly exercises the encoder, and
//! serialization exercises the envelope wire format. Real UltraHonk proving
//! is measured live by `crucible-prover benchmark --backend ultrahonk` and
//! the CI live suites instead — this crate stays toolchain-free so `cargo
//! bench` always runs.

#![forbid(unsafe_code)]

use std::path::Path;
use std::time::{Duration, Instant};

use crucible_vectors::TestVector;

/// Loads the repo's valid test vectors (one per operation) for benching.
pub fn valid_vectors() -> Vec<TestVector> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors");
    let mut vectors = crucible_vectors::load_catalog(&root).expect("catalog loads");
    vectors.retain(|v| v.expect_verification);
    vectors.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(vectors.len(), 5, "one valid vector per operation");
    vectors
}

/// Runs `op` once and returns the elapsed time.
pub fn time_once<F: FnMut()>(mut op: F) -> Duration {
    let start = Instant::now();
    op();
    start.elapsed()
}

/// Best-of-`samples` single-shot timing of `op`, in nanoseconds.
///
/// Single-shot timing (rather than a timed loop) keeps the mock's tiny
/// per-call costs measurable; taking the best sample discards scheduler
/// noise. Warmup runs `op` once so first-call effects are excluded.
pub fn best_ns<F: FnMut()>(samples: usize, mut op: F) -> f64 {
    op(); // warmup
    let mut best = f64::INFINITY;
    for _ in 0..samples {
        let ns = time_once(&mut op).as_secs_f64() * 1e9;
        best = best.min(ns);
    }
    best
}

/// Formats nanoseconds per operation as a short human string.
pub fn fmt_ns(ns: f64) -> String {
    if ns >= 1e6 {
        format!("{:>10.3} ms", ns / 1e6)
    } else if ns >= 1e3 {
        format!("{:>10.3} µs", ns / 1e3)
    } else {
        format!("{:>10.3} ns", ns)
    }
}
