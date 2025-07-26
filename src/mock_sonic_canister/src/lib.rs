use candid::Nat;
use candid::{CandidType, Principal};
use ic_cdk_macros::{query, update};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::Mutex;

#[derive(CandidType, Deserialize, Clone)]
struct Token {
    address: String,
    decimals: u8,
}

#[derive(CandidType, Deserialize, Clone)]
struct PositionInfo {
    token_a: Token,
    token_b: Token,
    token_a_amount: u64,
    token_b_amount: u64,
    reward_token: Token,
    reward_amount: u64,
    auto_compound: bool,
}

static HEIGHT: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));
static TOTAL_SUPPLY: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(10_000_000_000));
static TOTAL_REWARDS: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(50_000_000));

#[candid::candid_method(query)]
#[query]
fn get_user_positions(_p: Principal) -> Vec<PositionInfo> {
    vec![
        PositionInfo {
            token_a: Token {
                address: "sonic0".to_string(),
                decimals: 8,
            },
            token_b: Token {
                address: "sonic1".to_string(),
                decimals: 8,
            },
            token_a_amount: 1_000_000_000,
            token_b_amount: 2_000_000_000,
            reward_token: Token {
                address: "SNR".to_string(),
                decimals: 8,
            },
            reward_amount: 50_000_000,
            auto_compound: false,
        },
        PositionInfo {
            token_a: Token {
                address: "sonic2".to_string(),
                decimals: 8,
            },
            token_b: Token {
                address: "sonic3".to_string(),
                decimals: 8,
            },
            token_a_amount: 3_000_000_000,
            token_b_amount: 4_000_000_000,
            reward_token: Token {
                address: "SNR".to_string(),
                decimals: 8,
            },
            reward_amount: 0,
            auto_compound: true,
        },
    ]
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

#[candid::candid_method(update)]
#[update]
async fn claim(p: Principal, ledger: Principal) -> u64 {
    let _: () = ic_cdk::call(ledger, "credit", (p, Nat::from(25_000_000u64)))
        .await
        .unwrap();
    5_000
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
    Nat::from(50_000_000u64)
}

ic_cdk::export_candid!();
