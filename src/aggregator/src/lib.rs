pub mod cache;
pub mod cert;
pub mod cycles;
pub mod dex;
pub mod dex_fetchers;
pub mod error;
pub mod ledger_fetcher;
pub mod logging;
pub mod lp_cache;
pub mod metrics;
pub mod neuron_fetcher;
pub mod pool_registry;
pub mod utils;
pub mod warm;

use crate::utils::{now, MINUTE_NS};
use bx_core::Holding;
use candid::Principal;
use once_cell::sync::Lazy;
#[cfg(feature = "claim")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "claim")]
use std::sync::Mutex;

static MAX_HOLDINGS: Lazy<usize> = Lazy::new(|| {
    option_env!("MAX_HOLDINGS")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(500)
});
#[cfg(feature = "claim")]
static CLAIM_WALLETS: Lazy<HashSet<Principal>> = Lazy::new(|| {
    option_env!("CLAIM_WALLETS")
        .unwrap_or("")
        .split(',')
        .filter_map(|s| Principal::from_text(s.trim()).ok())
        .collect::<HashSet<_>>()
});
#[cfg(feature = "claim")]
static CLAIM_DENYLIST: Lazy<HashSet<Principal>> = Lazy::new(|| {
    option_env!("CLAIM_DENYLIST")
        .unwrap_or("")
        .split(',')
        .filter_map(|s| Principal::from_text(s.trim()).ok())
        .collect::<HashSet<_>>()
});
#[cfg(feature = "claim")]
static CLAIM_LOCKS: Lazy<Mutex<HashMap<Principal, u64>>> = Lazy::new(|| Mutex::new(HashMap::new()));
#[cfg(feature = "claim")]
static CLAIM_LOCK_TIMEOUT_NS: Lazy<u64> = Lazy::new(|| {
    option_env!("CLAIM_LOCK_TIMEOUT_SECS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(300)
        * 1_000_000_000u64
});

#[cfg(feature = "claim")]
static CLAIM_LIMIT_WINDOW_NS: Lazy<u64> = Lazy::new(|| {
    option_env!("CLAIM_LIMIT_WINDOW_SECS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(crate::utils::DAY_SECS)
        * 1_000_000_000u64
});

#[cfg(feature = "claim")]
static CLAIM_DAILY_LIMIT: Lazy<u32> = Lazy::new(|| {
    option_env!("CLAIM_DAILY_LIMIT")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(5)
});

#[cfg(feature = "claim")]
static CLAIM_MAX_TOTAL: Lazy<u64> = Lazy::new(|| {
    option_env!("CLAIM_MAX_TOTAL")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(u64::MAX)
});

#[cfg(feature = "claim")]
static MAX_CLAIM_PER_CALL: Lazy<usize> = Lazy::new(|| {
    option_env!("MAX_CLAIM_PER_CALL")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX)
});

#[cfg(feature = "claim")]
static CLAIM_COUNTS: Lazy<Mutex<HashMap<Principal, (u32, u64)>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(feature = "claim")]
static CLAIM_ADAPTER_TIMEOUT_SECS: Lazy<u64> = Lazy::new(|| {
    option_env!("CLAIM_ADAPTER_TIMEOUT_SECS")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10)
});

async fn calculate_holdings(principal: Principal) -> Vec<Holding> {
    let (ledger, neuron, dex) = futures::join!(
        ledger_fetcher::fetch(principal),
        neuron_fetcher::fetch(principal),
        dex_fetchers::fetch(principal)
    );

    let capacity =
        ledger.as_ref().map_or(0, |v| v.len()) + neuron.len() + dex.as_ref().map_or(0, |v| v.len());
    let mut holdings = Vec::with_capacity(capacity);
    holdings.extend(ledger.unwrap_or_default());
    holdings.extend(neuron);
    holdings.extend(dex.unwrap_or_default());
    if holdings.len() > *MAX_HOLDINGS {
        holdings.truncate(*MAX_HOLDINGS);
    }
    holdings
}

#[cfg(target_arch = "wasm32")]
fn instructions() -> u64 {
    ic_cdk::api::instruction_counter()
}

#[cfg(not(target_arch = "wasm32"))]
fn instructions() -> u64 {
    0
}

#[ic_cdk_macros::query]
pub async fn get_holdings(principal: Principal) -> Vec<Holding> {
    metrics::inc_query();
    let start = instructions();
    let now = now();
    {
        let cache = cache::get();
        if let Some(v) = cache.get(&principal) {
            let (cached, ts) = v.value().clone();
            if now - ts < MINUTE_NS {
                let used = instructions().saturating_sub(start);
                tracing::info!(
                    "get_holdings took {used} instructions ({:.2} B)",
                    used as f64 / 1_000_000_000f64
                );
                return cached;
            }
        }
    }

    let (ledger, neuron, dex) = futures::join!(
        ledger_fetcher::fetch(principal),
        neuron_fetcher::fetch(principal),
        dex_fetchers::fetch(principal)
    );

    let capacity =
        ledger.as_ref().map_or(0, |v| v.len()) + neuron.len() + dex.as_ref().map_or(0, |v| v.len());
    let mut holdings = Vec::with_capacity(capacity);
    holdings.extend(ledger.unwrap_or_default());
    holdings.extend(neuron);
    holdings.extend(dex.unwrap_or_default());

    {
        cache::get().insert(principal, (holdings.clone(), now));
    }
    let used = instructions().saturating_sub(start);
    tracing::info!(
        "get_holdings took {used} instructions ({:.2} B)",
        used as f64 / 1_000_000_000f64
    );
    holdings
}

#[cfg(feature = "claim")]
#[ic_cdk_macros::update]
pub async fn claim_all_rewards(principal: Principal) -> Vec<u64> {
    metrics::inc_query();
    metrics::inc_claim_attempt();
    let caller = ic_cdk::caller();
    if caller != principal && !CLAIM_WALLETS.contains(&caller) {
        ic_cdk::api::trap("unauthorized");
    }
    if principal == Principal::anonymous() {
        ic_cdk::api::trap("invalid principal");
    }
    if CLAIM_DENYLIST.contains(&principal) {
        ic_cdk::api::trap("denied");
    }
    {
        let mut counts = CLAIM_COUNTS.lock().unwrap();
        let now = now();
        let entry = counts
            .entry(principal)
            .or_insert((0, now + *CLAIM_LIMIT_WINDOW_NS));
        if now > entry.1 {
            *entry = (0, now + *CLAIM_LIMIT_WINDOW_NS);
        }
        if entry.0 >= *CLAIM_DAILY_LIMIT {
            ic_cdk::api::trap("claim limit reached");
        }
        entry.0 += 1;
    }
    {
        let mut locks = CLAIM_LOCKS.lock().unwrap();
        let now = now();
        locks.retain(|_, exp| *exp > now);
        if locks.contains_key(&principal) {
            ic_cdk::api::trap("claim already in progress");
        }
        locks.insert(principal, now + *CLAIM_LOCK_TIMEOUT_NS);
    }
    struct Guard(Principal);
    impl Drop for Guard {
        fn drop(&mut self) {
            CLAIM_LOCKS.lock().unwrap().remove(&self.0);
        }
    }
    let _guard = Guard(principal);
    use dex::{
        dex_icpswap::IcpswapAdapter, dex_infinity::InfinityAdapter, dex_sonic::SonicAdapter,
        sns_adapter::SnsAdapter, DexAdapter,
    };
    let mut adapters: Vec<Box<dyn DexAdapter>> = vec![
        Box::new(IcpswapAdapter),
        Box::new(SonicAdapter),
        Box::new(InfinityAdapter),
        Box::new(SnsAdapter),
    ];
    if *MAX_CLAIM_PER_CALL < adapters.len() {
        adapters.truncate(*MAX_CLAIM_PER_CALL);
    }
    let mut spent = Vec::with_capacity(adapters.len());
    let mut total: u64 = 0;
    for a in adapters {
        if total >= *CLAIM_MAX_TOTAL {
            break;
        }
        if let Some(c) = claim_with_timeout(a.claim_rewards(principal)).await {
            total = total.saturating_add(c);
            if total > *CLAIM_MAX_TOTAL {
                ic_cdk::api::trap("claim total exceeded");
            }
            spent.push(c);
        }
    }
    metrics::inc_claim_success();
    spent
}

#[cfg(feature = "claim")]
async fn claim_with_timeout<F>(fut: F) -> Option<u64>
where
    F: std::future::Future<Output = Result<u64, String>>,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        use tokio::time::{timeout, Duration};
        match timeout(Duration::from_secs(*CLAIM_ADAPTER_TIMEOUT_SECS), fut).await {
            Ok(Ok(v)) => Some(v),
            Ok(Err(e)) => {
                tracing::error!("claim failed: {e}");
                None
            }
            Err(_) => {
                tracing::error!("claim timed out");
                None
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        match fut.await {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!("claim failed: {e}");
                None
            }
        }
    }
}

#[ic_cdk_macros::query]
pub fn pools_graphql(query: String) -> String {
    metrics::inc_query();
    pool_registry::graphql(query)
}

#[derive(candid::CandidType, serde::Serialize)]
pub struct CertifiedHoldings {
    pub holdings: Vec<Holding>,
    #[serde(with = "serde_bytes")]
    pub certificate: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub witness: Vec<u8>,
}

#[ic_cdk_macros::update]
pub async fn refresh_holdings(principal: Principal) {
    metrics::inc_query();
    let now = now();
    let holdings = calculate_holdings(principal).await;
    cache::get().insert(principal, (holdings.clone(), now));
    cert::update(principal, &holdings);
}

#[ic_cdk_macros::query]
pub fn get_holdings_cert(principal: Principal) -> CertifiedHoldings {
    metrics::inc_query();
    let holdings = cache::get()
        .get(&principal)
        .map(|v| v.value().0.clone())
        .unwrap_or_default();
    let certificate = ic_cdk::api::data_certificate().unwrap_or_default();
    let witness = cert::witness(principal);
    CertifiedHoldings {
        holdings,
        certificate,
        witness,
    }
}

#[derive(candid::CandidType, serde::Serialize)]
pub struct Version {
    pub git_sha: &'static str,
    pub build_time: &'static str,
}

#[ic_cdk_macros::query]
pub fn get_version() -> Version {
    metrics::inc_query();
    Version {
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown"),
        build_time: option_env!("BUILD_TIME").unwrap_or("unknown"),
    }
}

#[ic_cdk_macros::query]
pub fn get_cycles_log() -> Vec<String> {
    metrics::inc_query();
    cycles::log()
}

#[cfg(feature = "claim")]
#[derive(candid::CandidType, serde::Serialize)]
pub struct ClaimStatus {
    pub attempts: u32,
    pub window_expires: u64,
    pub locked: bool,
}

#[cfg(feature = "claim")]
#[ic_cdk_macros::query]
pub fn get_claim_status(principal: Principal) -> ClaimStatus {
    metrics::inc_query();
    let now = now();
    let (attempts, window_expires) = CLAIM_COUNTS
        .lock()
        .unwrap()
        .get(&principal)
        .cloned()
        .unwrap_or((0, now + *CLAIM_LIMIT_WINDOW_NS));
    let locked = CLAIM_LOCKS
        .lock()
        .unwrap()
        .get(&principal)
        .is_some_and(|exp| *exp > now);
    ClaimStatus {
        attempts,
        window_expires,
        locked,
    }
}

#[ic_cdk_macros::query]
pub fn health_check() -> &'static str {
    metrics::inc_query();
    "ok"
}
