//! Witness-assembly bench: request → WitnessData → Prover.toml encoding.
//!
//! This is the step between a proof request and the toolchain: assembling
//! the private/public split and hex-encoding it exactly as Noir expects.
//! It is also the only place private values leave memory, so its cost
//! matters for high-throughput request building.

use crucible_benches::{best_ns, fmt_ns, valid_vectors};
use crucible_witness::{WitnessData, encoder};

fn main() {
    const SAMPLES: usize = 1000;
    println!("witness build (WitnessData + Prover.toml encode), best of {SAMPLES}");
    println!("{:<28} ns/op", "vector");

    for vector in valid_vectors() {
        let request = vector.to_request();
        let data = WitnessData::from_request(&request);
        let ns = best_ns(SAMPLES, || {
            let encoded = encoder::encode_toml(&data);
            assert!(encoded.contains("0x"));
        });
        println!("{:<28} {}", vector.id, fmt_ns(ns));
    }
}
