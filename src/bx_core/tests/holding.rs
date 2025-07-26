use bx_core::Holding;

#[test]
fn json_round_trip() {
    let holding = Holding {
        source: "ledger".into(),
        token: "ICP".into(),
        amount: "1.23".into(),
        status: "liquid".into(),
    };
    let json = serde_json::to_string(&holding).unwrap();
    let decoded: Holding = serde_json::from_str(&json).unwrap();
    assert_eq!(holding, decoded);
}
