//! Token identity.
//!
//! A confidential balance belongs to exactly one token. Every operation
//! circuit takes the token address as a public input and binds it into the
//! operation, so a proof for token A can never be submitted against token B.

/// A token's public address (an opaque identifier field).
pub struct Token {
    /// The token's address.
    pub address: Field,
}

impl Token {
    /// Wraps a token address.
    pub fn new(address: Field) -> Token {
        Token { address }
    }

    /// Asserts this token equals `address` (binds an operation to a token).
    pub fn assert_is(self, address: Field) {
        assert(self.address == address, "token mismatch");
    }
}

#[test]
fn token_binding_checks_equality() {
    Token::new(0xabc).assert_is(0xabc);
}
