//! Measurement gadget: a single range check at the production amount width.
//!
//! Every amount in an operation circuit must be constrained to
//! `AMOUNT_BITS` (63 bits) before it may participate in arithmetic. This
//! gadget reports the per-amount cost of that constraint, so the total
//! range budget of a circuit can be estimated from its number of amounts.

use circuit_lib::AMOUNT_BITS;
use circuit_lib::constraints::range::assert_in_bits;

fn main(value: Field) -> pub Field {
    assert_in_bits(value, AMOUNT_BITS);
    value
}

#[test]
fn gadget_accepts_in_range_values() {
    assert(main(0) == 0);
    assert(main(1000) == 1000);
}

#[test]
fn gadget_accepts_the_top_of_range() {
    let top = (1u64 << 63) - 1;
    assert(main(top as Field) == top as Field);
}
