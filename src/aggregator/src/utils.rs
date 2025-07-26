use candid::Nat;
#[cfg(not(target_arch = "wasm32"))]
use num_traits::cast::ToPrimitive;
#[cfg(not(target_arch = "wasm32"))]
use once_cell::sync::{Lazy, OnceCell};
#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::RwLock;

/// Common time constants in nanoseconds
pub const MINUTE_NS: u64 = 60_000_000_000;
pub const DAY_NS: u64 = 86_400_000_000_000;
pub const WEEK_NS: u64 = DAY_NS * 7;
pub const DAY_SECS: u64 = 86_400;
/// Seconds in one week
pub const WEEK_SECS: u64 = DAY_SECS * 7;
/// Maximum decimals supported for token formatting
pub const MAX_DECIMALS: u8 = 18;

#[cfg(not(target_arch = "wasm32"))]
pub const DEFAULT_LEDGER_URL: &str = "http://localhost:4943";

#[cfg(not(target_arch = "wasm32"))]
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_nanos() as u64
}

#[cfg(target_arch = "wasm32")]
pub fn now() -> u64 {
    ic_cdk::api::time()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn format_amount(n: Nat, decimals: u8) -> String {
    use num_bigint::BigUint;
    use num_integer::Integer;
    let div = BigUint::from(10u32).pow(decimals as u32);
    let (q, r) = n.0.div_rem(&div);
    let mut frac = r.to_str_radix(10);
    while frac.len() < decimals as usize {
        frac.insert(0, '0');
    }
    if decimals == 0 {
        q.to_str_radix(10)
    } else {
        format!("{}.{frac}", q.to_str_radix(10))
    }
}

#[cfg(target_arch = "wasm32")]
pub fn format_amount(n: Nat, _decimals: u8) -> String {
    n.0.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn idl_to_u64(val: &candid::types::value::IDLValue) -> Option<u64> {
    use candid::types::value::IDLValue;
    match val {
        IDLValue::Nat(n) => Some(n.0.to_u64().unwrap_or(0)),
        IDLValue::Nat8(n) => Some(*n as u64),
        IDLValue::Nat16(n) => Some(*n as u64),
        IDLValue::Nat32(n) => Some(*n as u64),
        IDLValue::Nat64(n) => Some(*n),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn idl_to_u8(val: &candid::types::value::IDLValue) -> Option<u8> {
    idl_to_u64(val).map(|v| v as u8)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn idl_to_string(val: &candid::types::value::IDLValue) -> Option<String> {
    use candid::types::value::IDLValue;
    match val {
        IDLValue::Text(s) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
static AGENT: OnceCell<ic_agent::Agent> = OnceCell::new();

#[cfg(not(target_arch = "wasm32"))]
static PRINCIPAL_CACHE: Lazy<RwLock<HashMap<String, Option<candid::Principal>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_agent() -> ic_agent::Agent {
    if let Some(a) = AGENT.get() {
        return a.clone();
    }
    let url = std::env::var("LEDGER_URL").unwrap_or_else(|_| DEFAULT_LEDGER_URL.to_string());
    let agent = ic_agent::Agent::builder()
        .with_url(url)
        .build()
        .expect("failed to build agent");
    if let Err(e) = agent.fetch_root_key().await {
        tracing::warn!("failed to fetch root key: {e}");
    }
    let _ = AGENT.set(agent.clone());
    agent
}

#[cfg(not(target_arch = "wasm32"))]
pub struct DexEntry {
    pub id: candid::Principal,
    pub controller: Option<candid::Principal>,
    pub enabled: bool,
}

#[cfg(not(target_arch = "wasm32"))]
static DEX_CONFIG: once_cell::sync::Lazy<
    std::sync::RwLock<std::collections::HashMap<String, DexEntry>>,
> = once_cell::sync::Lazy::new(|| std::sync::RwLock::new(std::collections::HashMap::new()));

#[cfg(not(target_arch = "wasm32"))]
static CONFIG_LOCK: once_cell::sync::Lazy<tokio::sync::Mutex<()>> =
    once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(()));

#[cfg(not(target_arch = "wasm32"))]
pub async fn load_dex_config() {
    use std::path::Path;
    use tracing::{info, warn};
    let _guard = CONFIG_LOCK.lock().await;

    let path = std::env::var("LEDGERS_FILE").unwrap_or_else(|_| "config/ledgers.toml".to_string());
    if !Path::new(&path).exists() {
        tracing::error!("dex config {path} missing");
        return;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let value: toml::Value =
        toml::from_str(&text).unwrap_or(toml::Value::Table(Default::default()));

    let dex_table = value
        .get("dex")
        .and_then(|d| d.as_table())
        .cloned()
        .unwrap_or_default();
    let ctrl_table = value
        .get("dex_controllers")
        .and_then(|d| d.as_table())
        .cloned()
        .unwrap_or_default();

    use std::collections::HashSet;
    let mut map = std::collections::HashMap::with_capacity(dex_table.len());
    let mut seen = HashSet::with_capacity(dex_table.len());
    for (name, val) in dex_table.iter() {
        if let Some(id_str) = val.as_str() {
            match candid::Principal::from_text(id_str) {
                Ok(id) => {
                    if !seen.insert(id) {
                        warn!("duplicate dex id {}", id);
                    }
                    let controller = ctrl_table
                        .get(name)
                        .and_then(|v| v.as_str())
                        .and_then(|s| candid::Principal::from_text(s).ok());
                    map.insert(
                        name.clone(),
                        DexEntry {
                            id,
                            controller,
                            enabled: true,
                        },
                    );
                }
                Err(e) => warn!("invalid dex id {id_str}: {e}"),
            }
        }
    }

    {
        let mut cfg = DEX_CONFIG.write().unwrap();
        *cfg = map;
    }

    // clear cached principals so updates take effect immediately
    PRINCIPAL_CACHE.write().unwrap().clear();

    for key in [
        "ICPSWAP_FACTORY",
        "SONIC_ROUTER",
        "INFINITY_VAULT",
        "SNS_DISTRIBUTOR",
    ] {
        if let Ok(val) = std::env::var(key) {
            match candid::Principal::from_text(&val) {
                Ok(p) => {
                    info!("{key} set; overriding ledgers.toml value");
                    if let Some(e) = DEX_CONFIG.write().unwrap().get_mut(key) {
                        e.id = p;
                        e.enabled = true;
                    } else {
                        DEX_CONFIG.write().unwrap().insert(
                            key.to_string(),
                            DexEntry {
                                id: p,
                                controller: None,
                                enabled: true,
                            },
                        );
                    }
                }
                Err(e) => warn!("{key} is not a valid principal: {e}"),
            }
        } else {
            info!("{key} not set; using ledgers.toml value");
        }
    }

    sanity_check_dex().await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn sanity_check_dex() {
    use tracing::error;
    let agent = get_agent().await;
    let names: Vec<String> = { DEX_CONFIG.read().unwrap().keys().cloned().collect() };
    for name in names {
        let id;
        let controller;
        {
            let cfg = DEX_CONFIG.read().unwrap();
            if let Some(e) = cfg.get(&name) {
                if !e.enabled {
                    continue;
                }
                id = e.id;
                controller = e.controller;
            } else {
                continue;
            }
        }
        let mut disable = false;
        if icrc1_metadata(&agent, id).await.is_none() {
            error!("{name} metadata failed; disabling adapter");
            disable = true;
        } else if let Some(c) = controller {
            if !controller_matches(&agent, id, c).await {
                error!("{name} controller mismatch; disabling adapter");
                disable = true;
            }
        }
        if disable {
            if let Some(e) = DEX_CONFIG.write().unwrap().get_mut(&name) {
                e.enabled = false;
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn icrc1_metadata(
    agent: &ic_agent::Agent,
    cid: candid::Principal,
) -> Option<Vec<(String, candid::types::value::IDLValue)>> {
    use candid::{Decode, Encode};
    let arg = Encode!().expect("encode args");
    let bytes = agent
        .query(&cid, "icrc1_metadata")
        .with_arg(arg)
        .call()
        .await
        .ok()?;
    Decode!(&bytes, Vec<(String, candid::types::value::IDLValue)>).ok()
}

#[cfg(not(target_arch = "wasm32"))]
async fn controller_matches(
    agent: &ic_agent::Agent,
    cid: candid::Principal,
    expected: candid::Principal,
) -> bool {
    use candid::{CandidType, Decode, Encode};
    use serde::Deserialize;

    #[derive(CandidType)]
    struct Req {
        canister_id: candid::Principal,
        num_requested_changes: Option<u64>,
    }

    #[derive(CandidType, Deserialize)]
    struct Resp {
        controllers: Vec<candid::Principal>,
    }

    let arg = Encode!(&Req {
        canister_id: cid,
        num_requested_changes: Some(0)
    })
    .unwrap();
    let bytes = match agent
        .query(&candid::Principal::management_canister(), "canister_info")
        .with_arg(arg)
        .call()
        .await
    {
        Ok(b) => b,
        Err(_) => return false,
    };
    let resp: Resp = match Decode!(&bytes, Resp) {
        Ok(v) => v,
        Err(_) => return false,
    };
    resp.controllers.contains(&expected)
}

#[cfg(not(target_arch = "wasm32"))]
static WATCHER: OnceCell<notify::RecommendedWatcher> = OnceCell::new();

#[cfg(not(target_arch = "wasm32"))]
pub fn watch_dex_config() {
    use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::path::Path;
    if WATCHER.get().is_some() {
        tracing::debug!("dex config watcher already running");
        return;
    }
    let path = std::env::var("LEDGERS_FILE").unwrap_or_else(|_| "config/ledgers.toml".to_string());
    if !Path::new(&path).exists() {
        tracing::error!("dex config {path} missing");
        return;
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(ev) = res {
                if matches!(ev.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    let _ = tx.send(());
                }
            }
        },
        notify::Config::default(),
    )
    .expect("watcher");
    if let Err(e) = watcher.watch(Path::new(&path), RecursiveMode::NonRecursive) {
        tracing::error!("failed to watch dex config: {e}");
        return;
    }
    let _ = WATCHER.set(watcher);
    tracing::info!("watching dex config at {}", path);
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            load_dex_config().await;
            crate::dex::clear_all_caches();
            crate::warm::init();
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn env_principal(name: &str) -> Option<candid::Principal> {
    if let Some(p) = PRINCIPAL_CACHE.read().unwrap().get(name) {
        return *p;
    }
    let val = DEX_CONFIG
        .read()
        .unwrap()
        .get(name)
        .filter(|e| e.enabled)
        .map(|e| e.id);
    PRINCIPAL_CACHE
        .write()
        .unwrap()
        .insert(name.to_string(), val);
    val
}

#[cfg(not(target_arch = "wasm32"))]
pub fn dex_ids() -> Vec<candid::Principal> {
    DEX_CONFIG
        .read()
        .unwrap()
        .values()
        .filter(|e| e.enabled)
        .map(|e| e.id)
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub fn env_principal(name: &str) -> Option<candid::Principal> {
    match name {
        "ICPSWAP_FACTORY" => {
            option_env!("ICPSWAP_FACTORY").and_then(|s| candid::Principal::from_text(s).ok())
        }
        "SONIC_ROUTER" => {
            option_env!("SONIC_ROUTER").and_then(|s| candid::Principal::from_text(s).ok())
        }
        "INFINITY_VAULT" => {
            option_env!("INFINITY_VAULT").and_then(|s| candid::Principal::from_text(s).ok())
        }
        "SNS_DISTRIBUTOR" => {
            option_env!("SNS_DISTRIBUTOR").and_then(|s| candid::Principal::from_text(s).ok())
        }
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
pub fn dex_ids() -> Vec<candid::Principal> {
    [
        "ICPSWAP_FACTORY",
        "SONIC_ROUTER",
        "INFINITY_VAULT",
        "SNS_DISTRIBUTOR",
    ]
    .into_iter()
    .filter_map(env_principal)
    .collect()
}

#[cfg(target_arch = "wasm32")]
pub async fn warm_icrc_metadata(cid: candid::Principal) {
    let _: Result<(Vec<(String, candid::types::value::IDLValue)>,), _> =
        ic_cdk::api::call::call(cid, "icrc1_metadata", ()).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn warm_icrc_metadata(cid: candid::Principal) {
    let agent = get_agent().await;
    let _ = icrc1_metadata(&agent, cid).await;
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn dex_block_height(agent: &ic_agent::Agent, cid: candid::Principal) -> Option<u64> {
    use candid::{Decode, Encode};
    let arg = Encode!().expect("encode args");
    let bytes = agent
        .query(&cid, "block_height")
        .with_arg(arg)
        .call()
        .await
        .ok()?;
    Decode!(&bytes, u64).ok()
}
