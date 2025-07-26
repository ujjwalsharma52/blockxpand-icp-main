use candid::{CandidType, Nat, Principal};
use ic_cdk_macros::{query, update};
use num_traits::cast::ToPrimitive;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

static BALANCES: Lazy<Mutex<HashMap<Principal, u64>>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(Principal::anonymous(), 1_000_000_000u64);
    Mutex::new(m)
});

#[candid::candid_method(query)]
#[query]
fn icrc1_metadata() -> Vec<(String, candid::types::value::IDLValue)> {
    vec![
        (
            "icrc1:symbol".to_string(),
            candid::types::value::IDLValue::Text("MOCK".to_string()),
        ),
        (
            "icrc1:decimals".to_string(),
            candid::types::value::IDLValue::Nat8(8),
        ),
        (
            "icrc1:fee".to_string(),
            candid::types::value::IDLValue::Nat(100u64.into()),
        ),
    ]
}

#[derive(CandidType, Deserialize)]
struct Account {
    owner: Principal,
    subaccount: Option<Vec<u8>>,
}

#[candid::candid_method(query)]
#[query]
fn icrc1_balance_of(account: Account) -> Nat {
    let map = BALANCES.lock().unwrap();
    let bal = map.get(&account.owner).cloned().unwrap_or_default();
    Nat::from(bal)
}

#[derive(CandidType, Deserialize)]
struct CreditArgs {
    owner: Principal,
    amount: Nat,
}

#[candid::candid_method(update)]
#[update]
async fn credit(owner: Principal, amount: Nat) {
    let mut map = BALANCES.lock().unwrap();
    let entry = map.entry(owner).or_insert(0);
    *entry += amount.0.to_u64().unwrap_or(0);
}

ic_cdk::export_candid!();
