//! Account identity and ownership.
//!
//! A Crucible account is identified by its public **address**, which is the
//! hash of its secret key. Knowing the secret is what lets an account
//! authorize operations; the address itself reveals nothing about the
//! secret. This mirrors the Confidential Token model where the ledger stores
//! public identity while ownership is proven in zero knowledge.
//!
//! # Scope honesty
//!
//! Real deployments bind the address to a nullifier key and a viewing key
//! separately; here the address is a single key-hash so the ownership
//! constraint can be exercised. Key derivation must be aligned with the real
//! specification.

use crate::crypto::hashing;

/// A public account identity (the ledger-facing address).
pub struct Address {
    /// The account's public address field.
    pub value: Field,
}

impl Address {
    /// Wraps an existing address value.
    pub fn new(value: Field) -> Address {
        Address { value }
    }

    /// Derives the public address from an account secret key.
    ///
    /// This is the *ownership* statement proven by register and by every
    /// operation that spends an account's confidential state.
    pub fn from_secret(secret: Field) -> Address {
        Address { value: hashing::key_hash([secret]) }
    }

    /// Asserts that `secret` is the key belonging to this address.
    pub fn assert_owned_by(self, secret: Field) {
        assert(
            self.value == Address::from_secret(secret).value,
            "account secret does not match the address",
        );
    }
}

/// An account with a known secret key. This type only ever exists as
/// private witness material inside a circuit.
pub struct Account {
    /// The public address.
    pub address: Address,
    /// The secret key. Never becomes a public input or output.
    pub secret: Field,
}

impl Account {
    /// Builds an account from its secret, deriving the address.
    pub fn from_secret(secret: Field) -> Account {
        Account { address: Address::from_secret(secret), secret }
    }

    /// The address of this account.
    pub fn address_value(self) -> Field {
        self.address.value
    }
}

#[test]
fn address_is_deterministic_per_secret() {
    assert(Address::from_secret(42).value == Address::from_secret(42).value);
    assert(Address::from_secret(42).value != Address::from_secret(43).value);
}

#[test]
fn owned_secret_passes_assertion() {
    Address::from_secret(7).assert_owned_by(7);
}
