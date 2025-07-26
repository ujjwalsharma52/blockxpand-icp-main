use crate::error::FetchError;
#[cfg(not(target_arch = "wasm32"))]
use crate::utils::format_amount;
use bx_core::Holding;
#[cfg(not(target_arch = "wasm32"))]
use candid::Nat;
use candid::Principal;
#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
use candid::{Decode, Encode};
#[cfg(not(target_arch = "wasm32"))]
use dashmap::DashMap;
#[cfg(not(target_arch = "wasm32"))]
use futures::future::join_all;
use once_cell::sync::Lazy;
use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use sha2::{Digest, Sha256};
#[cfg(not(target_arch = "wasm32"))]
use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::num::NonZeroU8;

// Metadata for each ledger is cached with an expiry and a stable hash.
// When a hash mismatch is detected, the entry is replaced so callers
// always see the latest token symbol, decimals, and transfer fee.

#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
use ic_agent::Agent;
#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
#[derive(Clone, Default)]
struct Agent;

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
use std::sync::Mutex;

#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
fn now() -> u64 {
    crate::utils::now()
}
#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
static TEST_NOW: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));
#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
fn now() -> u64 {
    *TEST_NOW.lock().unwrap()
}

#[derive(Deserialize)]
struct LedgersConfig {
    ledgers: std::collections::HashMap<String, String>,
}

#[cfg(target_arch = "wasm32")]
pub static LEDGERS: Lazy<Vec<Principal>> = Lazy::new(|| {
    let cfg: LedgersConfig =
        toml::from_str(include_str!("../../../config/ledgers.toml")).expect("invalid config");
    let mut ids: Vec<Principal> = cfg
        .ledgers
        .values()
        .map(|id| Principal::from_text(id).expect("invalid principal"))
        .collect();
    ids.sort();
    ids.dedup();
    ids
});

#[cfg(not(target_arch = "wasm32"))]
pub static LEDGERS: Lazy<Vec<Principal>> = Lazy::new(|| {
    let path = std::env::var("LEDGERS_FILE").unwrap_or_else(|_| "config/ledgers.toml".to_string());
    let text = std::fs::read_to_string(path).expect("cannot read ledgers.toml");
    let cfg: LedgersConfig = toml::from_str(&text).expect("invalid config");
    let mut ids: Vec<Principal> = cfg
        .ledgers
        .values()
        .map(|id| Principal::from_text(id).expect("invalid principal"))
        .collect();
    ids.sort();
    ids.dedup();
    ids
});

/// Duration that cached metadata remains valid (default 24h)
#[cfg(not(target_arch = "wasm32"))]
static META_TTL_NS: Lazy<u64> = Lazy::new(|| {
    option_env!("META_TTL_SECS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(crate::utils::DAY_SECS)
        * 1_000_000_000u64
});

#[cfg(not(target_arch = "wasm32"))]
static LEDGER_RETRY_LIMIT: Lazy<NonZeroU8> = Lazy::new(|| {
    NonZeroU8::new(
        option_env!("LEDGER_RETRY_LIMIT")
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(3),
    )
    .unwrap()
});

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
struct Meta {
    symbol: String,
    decimals: u8,
    fee: u64,
    hash: [u8; 32],
    expires: u64,
}
#[cfg(not(target_arch = "wasm32"))]
static META_CACHE: Lazy<DashMap<Principal, Meta>> = Lazy::new(DashMap::new);

#[derive(candid::CandidType, serde::Deserialize, serde::Serialize)]
pub struct StableMeta {
    cid: Principal,
    symbol: String,
    decimals: u8,
    fee: u64,
    hash: Vec<u8>,
    expires: u64,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stable_save() -> Vec<StableMeta> {
    META_CACHE
        .iter()
        .map(|e| StableMeta {
            cid: *e.key(),
            symbol: e.value().symbol.clone(),
            decimals: e.value().decimals,
            fee: e.value().fee,
            hash: e.value().hash.to_vec(),
            expires: e.value().expires,
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stable_restore(data: Vec<StableMeta>) {
    META_CACHE.clear();
    for m in data {
        let mut hash = [0u8; 32];
        if m.hash.len() == 32 {
            hash.copy_from_slice(&m.hash);
        }
        META_CACHE.insert(
            m.cid,
            Meta {
                symbol: m.symbol,
                decimals: m.decimals,
                fee: m.fee,
                hash,
                expires: m.expires,
            },
        );
    }
}

#[cfg(target_arch = "wasm32")]
pub fn stable_save() -> Vec<StableMeta> {
    Vec::new()
}

#[cfg(target_arch = "wasm32")]
pub fn stable_restore(_: Vec<StableMeta>) {}

#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
async fn with_retry<F, Fut, T>(mut f: F) -> Result<T, ic_agent::AgentError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ic_agent::AgentError>>,
{
    let mut delay = 100u64;
    for attempt in 0..LEDGER_RETRY_LIMIT.get() {
        match f().await {
            Ok(v) => return Ok(v),
            Err(_e) if attempt < LEDGER_RETRY_LIMIT.get() - 1 => {
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
fn encode_items(items: &[(String, candid::types::value::IDLValue)]) -> Vec<u8> {
    Encode!(&items).expect("encode items")
}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
fn encode_items(items: &[(String, candid::types::value::IDLValue)]) -> Vec<u8> {
    use std::fmt::Write;
    let mut s = String::new();
    for (k, v) in items {
        write!(&mut s, "{k}:{v:?};").expect("write to string");
    }
    s.into_bytes()
}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
async fn with_retry<F, Fut, T>(mut f: F) -> Result<T, ic_agent::AgentError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, ic_agent::AgentError>>,
{
    for attempt in 0..LEDGER_RETRY_LIMIT.get() {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt + 1 == LEDGER_RETRY_LIMIT.get() => return Err(e),
            Err(_) => continue,
        }
    }
    unreachable!()
}

#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
async fn get_agent() -> Agent {
    crate::utils::get_agent().await
}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
async fn get_agent() -> Agent {
    Agent::default()
}

#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
async fn icrc1_metadata(
    agent: &Agent,
    canister_id: Principal,
) -> Result<Vec<(String, candid::types::value::IDLValue)>, ic_agent::AgentError> {
    let arg = candid::Encode!().expect("encode args");
    let bytes = agent
        .query(&canister_id, "icrc1_metadata")
        .with_arg(arg)
        .call()
        .await?;
    let res: Vec<(String, candid::types::value::IDLValue)> =
        candid::Decode!(&bytes, Vec<(String, candid::types::value::IDLValue)>)
            .expect("decode metadata");
    Ok(res)
}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
static MOCK_METADATA: Lazy<Mutex<Result<Vec<(String, candid::types::value::IDLValue)>, String>>> =
    Lazy::new(|| Mutex::new(Ok(vec![])));

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
async fn icrc1_metadata(
    _agent: &Agent,
    _canister_id: Principal,
) -> Result<Vec<(String, candid::types::value::IDLValue)>, ic_agent::AgentError> {
    match MOCK_METADATA.lock().unwrap().clone() {
        Ok(v) => Ok(v),
        Err(e) => Err(ic_agent::AgentError::MessageError(e)),
    }
}

#[cfg(all(any(not(test), feature = "live-test"), not(target_arch = "wasm32")))]
async fn icrc1_balance_of(
    agent: &Agent,
    canister_id: Principal,
    owner: Principal,
) -> Result<Nat, ic_agent::AgentError> {
    #[derive(candid::CandidType)]
    struct Account {
        owner: Principal,
        subaccount: Option<Vec<u8>>,
    }
    let arg = candid::Encode!(&Account {
        owner,
        subaccount: None
    })
    .expect("encode args");
    let bytes = agent
        .query(&canister_id, "icrc1_balance_of")
        .with_arg(arg)
        .call()
        .await?;
    let res: Nat = candid::Decode!(&bytes, Nat).expect("decode balance");
    Ok(res)
}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
static MOCK_BALANCE: Lazy<Mutex<Result<Nat, String>>> =
    Lazy::new(|| Mutex::new(Ok(Nat::from(0u32))));

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
async fn icrc1_balance_of(
    _agent: &Agent,
    _canister_id: Principal,
    _owner: Principal,
) -> Result<Nat, ic_agent::AgentError> {
    match MOCK_BALANCE.lock().unwrap().clone() {
        Ok(v) => Ok(v),
        Err(e) => Err(ic_agent::AgentError::MessageError(e)),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch(principal: Principal) -> Result<Vec<Holding>, FetchError> {
    let agent = get_agent().await;
    let mut ids: Vec<Principal> = LEDGERS.iter().cloned().collect();
    ids.sort();
    let futures = ids.into_iter().map(|cid| {
        let agent = agent.clone();
        async move {
            let (symbol, decimals, _) = fetch_metadata(&agent, cid).await?;
            let nat = with_retry(|| icrc1_balance_of(&agent, cid, principal))
                .await
                .map_err(FetchError::from)?;
            Ok::<Holding, FetchError>(Holding {
                source: "ledger".into(),
                token: symbol,
                amount: format_amount(nat, decimals),
                status: "liquid".into(),
            })
        }
    });
    let results = join_all(futures).await;
    let mut holdings = Vec::with_capacity(LEDGERS.len());
    for r in results {
        match r {
            Ok(h) => holdings.push(h),
            Err(e) => return Err(e),
        }
    }
    Ok(holdings)
}

#[cfg(target_arch = "wasm32")]
pub async fn fetch(_principal: Principal) -> Result<Vec<Holding>, FetchError> {
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_metadata(agent: &Agent, cid: Principal) -> Result<(String, u8, u64), FetchError> {
    if let Some(meta) = META_CACHE.get(&cid) {
        if meta.expires > now() {
            return Ok((meta.symbol.clone(), meta.decimals, meta.fee));
        }
    }
    let items = with_retry(|| icrc1_metadata(agent, cid))
        .await
        .map_err(FetchError::from)?;
    let encoded = encode_items(&items);
    let hash: [u8; 32] = Sha256::digest(&encoded).into();
    if let Some(meta) = META_CACHE.get(&cid) {
        if meta.hash == hash {
            META_CACHE.insert(
                cid,
                Meta {
                    hash,
                    expires: now() + *META_TTL_NS,
                    ..meta.clone()
                },
            );
            return Ok((meta.symbol.clone(), meta.decimals, meta.fee));
        }
    }
    let mut symbol = String::new();
    let mut decimals: u8 = 0;
    let mut fee: u64 = 0;
    for (k, v) in items {
        use candid::types::value::IDLValue::Text;
        match k.as_str() {
            "icrc1:symbol" => {
                if let Text(s) = v {
                    symbol = s;
                }
            }
            "icrc1:decimals" => {
                if let Some(d) = crate::utils::idl_to_u8(&v) {
                    decimals = d.min(crate::utils::MAX_DECIMALS);
                }
            }
            "icrc1:fee" => {
                if let Some(f) = crate::utils::idl_to_u64(&v) {
                    fee = f;
                }
            }
            _ => {}
        }
    }
    META_CACHE.insert(
        cid,
        Meta {
            symbol: symbol.clone(),
            decimals,
            fee,
            hash,
            expires: now() + *META_TTL_NS,
        },
    );
    Ok((symbol, decimals, fee))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn warm_metadata(cid: Principal) {
    let agent = get_agent().await;
    let _ = fetch_metadata(&agent, cid).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn warm_metadata(_cid: Principal) {}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
pub(super) fn set_mock_metadata(
    resp: Result<Vec<(String, candid::types::value::IDLValue)>, String>,
) {
    *MOCK_METADATA.lock().unwrap() = resp;
}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
pub(super) fn set_mock_balance(resp: Result<Nat, String>) {
    *MOCK_BALANCE.lock().unwrap() = resp;
}

#[cfg(all(test, not(feature = "live-test"), not(target_arch = "wasm32")))]
pub(super) fn set_now(value: u64) {
    *TEST_NOW.lock().unwrap() = value;
}

#[cfg(all(test, not(feature = "live-test")))]
mod tests {
    use super::*;
    use candid::types::value::IDLValue;

    #[test]
    fn format_amount_basic() {
        assert_eq!(format_amount(Nat::from(1000u64), 0), "1000");
        assert_eq!(format_amount(Nat::from(12345u64), 2), "123.45");
        assert_eq!(format_amount(Nat::from(5u64), 3), "0.005");
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial_test::serial]
    async fn metadata_caching_and_expiry() {
        let cid = Principal::from_text("aaaaa-aa").unwrap();
        let agent = get_agent().await;

        set_now(1);
        set_mock_metadata(Ok(vec![
            ("icrc1:symbol".into(), IDLValue::Text("AAA".into())),
            ("icrc1:decimals".into(), IDLValue::Nat8(2)),
            ("icrc1:fee".into(), IDLValue::Nat(Nat::from(10u64))),
        ]));
        META_CACHE.clear();
        let v1 = fetch_metadata(&agent, cid).await.unwrap();
        assert_eq!(v1, ("AAA".into(), 2, 10));

        set_now(2);
        set_mock_metadata(Ok(vec![
            ("icrc1:symbol".into(), IDLValue::Text("BBB".into())),
            ("icrc1:decimals".into(), IDLValue::Nat8(3)),
            ("icrc1:fee".into(), IDLValue::Nat(Nat::from(20u64))),
        ]));
        let v2 = fetch_metadata(&agent, cid).await.unwrap();
        assert_eq!(v2, ("AAA".into(), 2, 10));
        assert_eq!(META_CACHE.get(&cid).unwrap().symbol, "AAA");

        set_now(*META_TTL_NS + 3);
        let v3 = fetch_metadata(&agent, cid).await.unwrap();
        assert_eq!(v3, ("BBB".into(), 3, 20));
        assert_eq!(META_CACHE.get(&cid).unwrap().symbol, "BBB");
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial_test::serial]
    async fn metadata_error() {
        let cid = Principal::from_text("aaaaa-aa").unwrap();
        let agent = get_agent().await;
        set_now(0);
        set_mock_metadata(Err("fail".into()));
        META_CACHE.clear();
        let err = fetch_metadata(&agent, cid).await.unwrap_err();
        assert!(matches!(err, FetchError::Network(_)));
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial_test::serial]
    async fn with_retry_succeeds_after_retries() {
        let mut attempts = 0u8;
        let result = with_retry(|| {
            attempts += 1;
            async move {
                if attempts < 3 {
                    Err(ic_agent::AgentError::MessageError("no".into()))
                } else {
                    Ok(5)
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(result, 5);
        assert_eq!(attempts, 3);
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial_test::serial]
    async fn fetch_happy_path() {
        std::env::set_var("LEDGERS_FILE", "tests/ledgers_single.toml");
        once_cell::sync::Lazy::force(&LEDGERS);
        set_mock_metadata(Ok(vec![
            ("icrc1:symbol".into(), IDLValue::Text("AAA".into())),
            ("icrc1:decimals".into(), IDLValue::Nat8(2)),
        ]));
        set_mock_balance(Ok(Nat::from(1234u64)));
        META_CACHE.clear();
        let principal = Principal::from_text("aaaaa-aa").unwrap();
        let res = fetch(principal).await.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].token, "AAA");
        assert_eq!(res[0].amount, "12.34");
        assert_eq!(res[0].status, "liquid");
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial_test::serial]
    async fn fetch_balance_error() {
        std::env::set_var("LEDGERS_FILE", "tests/ledgers_single.toml");
        once_cell::sync::Lazy::force(&LEDGERS);
        set_mock_metadata(Ok(vec![
            ("icrc1:symbol".into(), IDLValue::Text("AAA".into())),
            ("icrc1:decimals".into(), IDLValue::Nat8(2)),
        ]));
        set_mock_balance(Err("oops".into()));
        META_CACHE.clear();
        let principal = Principal::from_text("aaaaa-aa").unwrap();
        let err = fetch(principal).await.unwrap_err();
        assert!(matches!(err, FetchError::Network(_)));
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial_test::serial]
    async fn fetch_metadata_error() {
        std::env::set_var("LEDGERS_FILE", "tests/ledgers_single.toml");
        once_cell::sync::Lazy::force(&LEDGERS);
        set_mock_metadata(Err("bad".into()));
        set_mock_balance(Ok(Nat::from(10u64)));
        META_CACHE.clear();
        let principal = Principal::from_text("aaaaa-aa").unwrap();
        let err = fetch(principal).await.unwrap_err();
        assert!(matches!(err, FetchError::Network(_)));
    }
}
