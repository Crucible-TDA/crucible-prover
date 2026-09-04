//! Measurement gadget: a single Pedersen commitment.
//!
//! `nargo info` reports the constraint cost of committing one
//! `(amount, blinding)` pair — the primitive every operation circuit is
//! built on. Tracking it in isolation lets a change to the commitment
//! scheme be costed before it propagates to the full circuits.

use circuit_lib::crypto::commit;

fn main(amount: Field, blinding: Field) -> pub Field {
    commit(amount, blinding)
}

#[test]
fn gadget_commits_deterministically() {
    assert(main(100, 7) == commit(100, 7));
}

#[test]
fn gadget_output_changes_with_inputs() {
    assert(main(100, 7) != main(101, 7));
    assert(main(100, 7) != main(100, 8));
}
