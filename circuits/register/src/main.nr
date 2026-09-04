//! Register circuit: prove ownership of a new account's secret key.
//!
//! Registration creates a ledger account identified by a public address.
//! The circuit proves knowledge of the secret key that derives to that
//! address — without revealing the key — so the ledger can safely record the
//! address as belonging to whoever holds the secret.
//!
//! ```text
//! public:  account_address
//! private: account_sk
//! proves:  account_address == key_hash([account_sk])
//! ```
//!
//! # What this circuit does not prove
//!
//! Address *uniqueness* (the address was not registered before) is a global
//! ledger fact and cannot be proven locally; the ledger rejects duplicate
//! registrations outside the circuit.

use circuit_lib::crypto::hashing;
use circuit_lib::types::account::Address;

fn main(account_address: pub Field, account_sk: Field) {
    // The address is the key hash; only the true secret passes.
    Address::new(account_address).assert_owned_by(account_sk);
}

#[test]
fn honest_registration_passes() {
    let sk = 12345;
    let address = Address::from_secret(sk).value;
    main(address, sk);
}

#[test]
fn registration_derivation_is_stable() {
    let address_a = Address::from_secret(7).value;
    let address_b = Address::from_secret(7).value;
    assert(address_a == address_b);
    assert(address_a != Address::from_secret(8).value);
}

#[test]
fn address_never_reveals_the_secret() {
    // key_hash is one-way: the public address must not equal the secret or
    // any trivial encoding of it (checked here for the fixture values).
    let sk = 12345;
    let address = Address::from_secret(sk).value;
    assert(address != sk);
    assert(hashing::pedersen_hash_fields([sk]) != address);
}
