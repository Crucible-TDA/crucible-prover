//! Range constraints over confidential amounts.
//!
//! Amounts live as `Field` elements, but a field can wrap the modulus, so a
//! circuit must *constrain* an amount to a small range before arithmetic on
//! it is safe. Casting to a bounded integer type (`u64`) makes the compiler
//! emit the range constraints; amount arithmetic in the operation circuits
//! then happens on those bounded integers.

/// Maximum amount width supported by the range helpers (u64). Values that
/// participate in sums use a smaller width (see `AMOUNT_BITS` in the crate
/// root) so additions cannot overflow.
pub global MAX_BITS: u64 = 64;

/// Asserts that `value` fits in `bits` (i.e. `0 <= value < 2^bits`).
///
/// The cast to `u64` is what generates the range proof: if `value` does not
/// fit the width, the cast is unsatisfiable and the proof fails.
pub fn assert_in_bits(value: Field, bits: u64) {
    assert(bits <= MAX_BITS, "range width exceeds u64");
    // The cast itself constrains value to < 2^64; for the full 64-bit width
    // that is the entire range, so no explicit bound is needed (shifting
    // 1u64 by 64 would overflow).
    let bounded: u64 = value as u64;
    if bits < 64 {
        assert(bounded < (1u64 << bits), "value exceeds the allowed range");
    }
}

/// Asserts `spend <= balance` in 64-bit space.
///
/// This is the overdraw guard: a transfer or withdrawal can only take as
/// much as the opening shows is there. Both values must already be
/// range-constrained (see [`assert_in_bits`]) so the subtraction cannot
/// wrap.
pub fn assert_no_overdraw(balance: Field, spend: Field) {
    let balance64: u64 = balance as u64;
    let spend64: u64 = spend as u64;
    assert(spend64 <= balance64, "insufficient balance for the operation");
    let _remainder: u64 = balance64 - spend64;
}

#[test]
fn small_values_pass_range_check() {
    assert_in_bits(0, 64);
    assert_in_bits(1000, 64);
}

#[test]
fn spend_within_balance_passes() {
    assert_no_overdraw(500, 100);
    assert_no_overdraw(500, 500);
}
