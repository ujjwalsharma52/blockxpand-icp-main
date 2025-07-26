use candid::{CandidType, Nat, Principal};
use ic_cdk_macros::{query, update};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::Mutex;

#[derive(CandidType, Deserialize, Clone)]
struct Position {
    ledger: Principal,
    subaccount: Vec<u8>,
}

static HEIGHT: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));
static TOTAL_SUPPLY: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(1_000_000_000));
static TOTAL_REWARDS: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));

#[candid::candid_method(query)]
#[query]
fn get_user_positions(_p: Principal) -> Vec<Position> {
    vec![Position {
        ledger: ic_cdk::api::id(),
        subaccount: vec![0; 32],
    }]
}

#[candid::candid_method(query)]
#[query]
fn icrc1_metadata() -> Vec<(String, candid::types::value::IDLValue)> {
    vec![
        (
            "icrc1:symbol".to_string(),
            candid::types::value::IDLValue::Text("INF".to_string()),
        ),
        (
            "icrc1:decimals".to_string(),
            candid::types::value::IDLValue::Nat8(8),
        ),
    ]
}

#[candid::candid_method(query)]
#[query]
fn icrc1_balance_of(_a: (Principal, Option<Vec<u8>>)) -> Nat {
    Nat::from(1_000_000_000u64)
}

#[candid::candid_method(query)]
#[query]
fn block_height() -> u64 {
    *HEIGHT.lock().unwrap()
}

#[candid::candid_method(update)]
#[update]
fn advance_block() {
    let mut h = HEIGHT.lock().unwrap();
    *h += 1;
}

#[candid::candid_method(query)]
#[query]
fn lp_total_supply() -> Nat {
    Nat::from(*TOTAL_SUPPLY.lock().unwrap())
}

#[candid::candid_method(query)]
#[query]
fn total_rewards() -> Nat {
    Nat::from(*TOTAL_REWARDS.lock().unwrap())
}

#[candid::candid_method(query)]
#[query]
fn claimable_rewards(_p: Principal) -> Nat {
    Nat::from(0u64)
}

ic_cdk::export_candid!();
