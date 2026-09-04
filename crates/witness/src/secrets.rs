//! Secret-handling policy and redaction helpers.
//!
//! # Policy
//!
//! The following must never reach logs, panic messages, debug output, CI
//! artifacts, public fixtures, or error messages:
//!
//! - private witness values ([`SecretValue`](crucible_interfaces::SecretValue))
//! - private randomness and secret openings
//! - private balances or amounts
//! - proving secret material / witness-derived values
//!
//! This is enforced in three layers: types that cannot format or serialize
//! themselves, structured errors that only carry names, and — for anything
//! that slips through as free-form text — the scrubbing helpers below.

/// Minimum run length of hex characters that gets scrubbed.
///
/// Short hex fragments (ids like `vk-transfer`, version suffixes) are
/// harmless; 16+ hex chars is what witness material looks like and is always
/// treated as sensitive.
pub const MIN_SCRUB_LEN: usize = 16;

/// Replaces every run of 16+ ASCII hex characters with `***`.
///
/// Use this as a last line of defense when formatting free-form text that
/// could contain witness material (CLI output, tool stderr, diagnostics).
/// Long hex is the shape secrets take in this codebase, so scrubbing it is a
/// cheap and effective guard.
pub fn scrub_hex(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_hexdigit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i - start >= MIN_SCRUB_LEN {
                out.push_str("***");
            } else {
                out.extend(&chars[start..i]);
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_hex_is_scrubbed_short_hex_is_kept() {
        let long = "deadbeefcafebabe1234567890abcdef";
        assert_eq!(scrub_hex(&format!("secret={long}")), "secret=***");
        // 16+ chars, exactly at the boundary
        assert_eq!(scrub_hex("0123456789abcdef"), "***");
        // Short fragments survive (ids, small field values in messages).
        assert_eq!(
            scrub_hex("vk-transfer-0.1.0 abc123"),
            "vk-transfer-0.1.0 abc123"
        );
        // Case-insensitive detection.
        assert_eq!(scrub_hex("DEADBEEFCAFEBABE1234567890ABCDEF"), "***");
    }

    #[test]
    fn scrub_keeps_normal_text_intact() {
        assert_eq!(
            scrub_hex("circuit transfer compiled ok"),
            "circuit transfer compiled ok"
        );
    }
}
