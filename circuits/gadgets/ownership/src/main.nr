//! Measurement gadget: the ownership assertion for one address.
//!
//! Every operation that spends an account's confidential state must prove
//! the prover knows the account secret for the public address (key
//! derivation + equality). This gadget costs that single assertion in
//! isolation.

use circuit_lib::types::account::Address;

fn main(address: pub Field, account_sk: Field) -> pub Field {
    Address::new(address).assert_owned_by(account_sk);
    address
}

#[test]
fn gadget_passes_for_the_real_owner() {
    let sk = 0x1234;
    let addr = Address::from_secret(sk).value;
    assert(main(addr, sk) == addr);
}
