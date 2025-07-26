use aggregator::dex::sns_adapter::{self, sns_claim, sns_get_claimable, Claimable};
use candid::{Nat, Principal};

#[tokio::test]
async fn sns_get_claimable_mock() {
    sns_adapter::test_helpers::set_claimable(Ok(vec![Claimable {
        symbol: "AAA".into(),
        amount: Nat::from(1234u64),
        decimals: 2,
    }]));
    let agent = ic_agent::Agent::builder()
        .with_url("http://127.0.0.1:0")
        .build()
        .unwrap();
    let res = sns_get_claimable(&agent, Principal::anonymous(), Principal::anonymous())
        .await
        .unwrap();
    assert_eq!(res.len(), 1);
}

#[tokio::test]
async fn sns_claim_mock() {
    sns_adapter::test_helpers::set_claim(Err("fail".into()));
    let agent = ic_agent::Agent::builder()
        .with_url("http://127.0.0.1:0")
        .build()
        .unwrap();
    let err = sns_claim(&agent, Principal::anonymous(), Principal::anonymous())
        .await
        .unwrap_err();
    assert!(matches!(err, ic_agent::AgentError::MessageError(_)));
}
