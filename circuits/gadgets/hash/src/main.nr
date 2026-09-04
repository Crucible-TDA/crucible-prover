//! Measurement gadget: one hash of two fields.
//!
//! Hashing is the second fundamental primitive (addresses, nullifiers,
//! domain separation all reduce to it). Measuring it alone answers whether a
//! cheaper hash or tighter domain layout is worth adopting before it is
//! wired into every circuit.

use circuit_lib::crypto::hashing::pedersen_hash_fields;

fn main(a: Field, b: Field) -> pub Field {
    pedersen_hash_fields([a, b])
}

#[test]
fn gadget_hashes_deterministically() {
    assert(main(1, 2) == pedersen_hash_fields([1, 2]));
}

#[test]
fn gadget_output_changes_with_inputs() {
    assert(main(1, 2) != main(2, 1));
}
