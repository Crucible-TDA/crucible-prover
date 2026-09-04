use crate::circuit::PublicInputBag;

/// The public outputs a proof binds itself to.
///
/// Circuit outputs are the values a verifier can check without knowing the
/// witness: the new commitment, the sender/recipient identities, the state
/// root the transition applies to, and so on. They share the value model of
/// [`PublicInputBag`] — ordered, canonical, serializable — and are expressed
/// as a type alias so the *role* of the values (outputs of a circuit, as
/// opposed to inputs of a request) stays explicit at the call site.
pub type OutputBag = PublicInputBag;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit::FieldValue;

    #[test]
    fn output_bag_is_a_public_input_bag() {
        let mut out = OutputBag::new();
        out.insert("new_commitment", FieldValue::from_hex("c0ffee").unwrap())
            .unwrap();
        assert_eq!(out.len(), 1);
        // Outputs serialize exactly like public inputs (they are public).
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains("c0ffee"));
    }
}
