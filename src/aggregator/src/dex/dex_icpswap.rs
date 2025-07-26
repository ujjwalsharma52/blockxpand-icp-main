use super::DexAdapter;
use crate::error::FetchError;
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    lp_cache,
    utils::{format_amount, get_agent, now},
};
use async_trait::async_trait;
use bx_core::Holding;
use candid::{CandidType, Nat, Principal};
#[cfg(not(target_arch = "wasm32"))]
use candid::{Decode, Encode};
#[cfg(not(target_arch = "wasm32"))]
use dashmap::DashMap;
#[cfg(not(target_arch = "wasm32"))]
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
struct Token {
    address: String,
    standard: String,
}

#[derive(CandidType, Deserialize)]
struct PoolData {
    key: String,
    token0: Token,
    token1: Token,
    fee: Nat,
    #[serde(rename = "tickSpacing")]
    tick_spacing: i32,
    #[serde(rename = "canisterId")]
    canister_id: Principal,
}

#[derive(CandidType, Deserialize)]
struct UserPositionInfoWithTokenAmount {
    #[serde(rename = "id")]
    id: Nat,
    #[serde(rename = "token0Amount")]
    token0_amount: Nat,
    #[serde(rename = "token1Amount")]
    token1_amount: Nat,
}

#[derive(CandidType, Deserialize, Clone)]
struct PoolMetadata {
    token0_decimals: u8,
    token1_decimals: u8,
}

#[cfg(not(target_arch = "wasm32"))]
static META_CACHE: Lazy<DashMap<Principal, (PoolMetadata, u64)>> = Lazy::new(DashMap::new);
#[cfg(not(target_arch = "wasm32"))]
const META_TTL_NS: u64 = crate::utils::DAY_NS; // 24h

#[async_trait]
impl DexAdapter for IcpswapAdapter {
    async fn fetch_positions(&self, principal: Principal) -> Result<Vec<Holding>, FetchError> {
        fetch_positions_impl(principal).await
    }

    #[cfg(feature = "claim")]
    async fn claim_rewards(&self, principal: Principal) -> Result<u64, String> {
        claim_rewards_impl(principal).await
    }
}

pub struct IcpswapAdapter;

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_cache() {
    META_CACHE.clear();
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_positions_impl(principal: Principal) -> Result<Vec<Holding>, FetchError> {
    let factory_id = match crate::utils::env_principal("ICPSWAP_FACTORY") {
        Some(p) => p,
        None => return Err(FetchError::InvalidConfig("factory".into())),
    };
    let agent = get_agent().await;
    let arg = Encode!().map_err(|_| FetchError::InvalidResponse)?;
    let bytes = match agent
        .query(&factory_id, "getPools")
        .with_arg(arg)
        .call()
        .await
    {
        Ok(b) => b,
        Err(e) => return Err(FetchError::from(e)),
    };
    let pools: Vec<PoolData> =
        Decode!(&bytes, Vec<PoolData>).map_err(|_| FetchError::InvalidResponse)?;
    let mut out = Vec::with_capacity(pools.len() * 3);
    for pool in pools.iter() {
        let height = crate::utils::dex_block_height(&agent, pool.canister_id)
            .await
            .unwrap_or(0);
        let pool_key = pool.key.clone();
        let holdings = lp_cache::get_or_fetch(principal, &pool_key, height, || async {
            let positions: Vec<UserPositionInfoWithTokenAmount> =
                query_positions(&agent, pool.canister_id, principal)
                    .await
                    .unwrap_or_default();
            let meta = match fetch_meta(&agent, pool.canister_id).await {
                Some(m) => m,
                None => return Vec::new(),
            };
            let mut temp = Vec::with_capacity(positions.len() * 3);
            for pos in positions {
                let a0 = format_amount(pos.token0_amount, meta.token0_decimals);
                temp.push(Holding {
                    source: "ICPSwap".into(),
                    token: pool.token0.address.clone(),
                    amount: a0,
                    status: "lp_escrow".into(),
                });
                let a1 = format_amount(pos.token1_amount, meta.token1_decimals);
                temp.push(Holding {
                    source: "ICPSwap".into(),
                    token: pool.token1.address.clone(),
                    amount: a1,
                    status: "lp_escrow".into(),
                });
            }
            temp
        })
        .await;
        out.extend(holdings);
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_positions_impl(_principal: Principal) -> Result<Vec<Holding>, FetchError> {
    Ok(Vec::new())
}

#[cfg(not(target_arch = "wasm32"))]
async fn query_positions(
    agent: &ic_agent::Agent,
    cid: Principal,
    owner: Principal,
) -> Option<Vec<UserPositionInfoWithTokenAmount>> {
    let arg = Encode!(&owner).ok()?;
    let bytes = agent
        .query(&cid, "get_user_positions_by_principal")
        .with_arg(arg)
        .call()
        .await
        .ok()?;
    Decode!(&bytes, Vec<UserPositionInfoWithTokenAmount>).ok()
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_meta(agent: &ic_agent::Agent, cid: Principal) -> Option<PoolMetadata> {
    if let Some(entry) = META_CACHE.get(&cid) {
        if entry.value().1 > now() {
            return Some(entry.value().0.clone());
        }
    }
    let arg = Encode!().ok()?;
    let bytes = agent
        .query(&cid, "metadata")
        .with_arg(arg)
        .call()
        .await
        .ok()?;
    let meta: PoolMetadata = Decode!(&bytes, PoolMetadata).ok()?;
    META_CACHE.insert(cid, (meta.clone(), now() + META_TTL_NS));
    Some(meta)
}

#[cfg(all(feature = "claim", not(target_arch = "wasm32")))]
async fn claim_rewards_impl(principal: Principal) -> Result<u64, String> {
    use crate::cache;
    let factory_id = match crate::utils::env_principal("ICPSWAP_FACTORY") {
        Some(p) => p,
        None => return Err("factory".into()),
    };
    let ledger = crate::ledger_fetcher::LEDGERS
        .first()
        .cloned()
        .ok_or("ledger")?;
    let agent = get_agent().await;
    let arg = Encode!().map_err(|_| "encode")?;
    let bytes = agent
        .query(&factory_id, "getPools")
        .with_arg(arg)
        .call()
        .await
        .map_err(|e| e.to_string())?;
    let pools: Vec<PoolData> = Decode!(&bytes, Vec<PoolData>).map_err(|_| "invalid response")?;
    let mut total: u64 = 0;
    for pool in pools {
        let arg = Encode!(&principal, &ledger).map_err(|e| e.to_string())?;
        let bytes = agent
            .update(&pool.canister_id, "claim")
            .with_arg(arg)
            .call_and_wait()
            .await
            .map_err(|e| e.to_string())?;
        let spent: u64 = Decode!(&bytes, u64).map_err(|_| "invalid response")?;
        total = total.checked_add(spent).ok_or("overflow")?;
    }
    // refresh cache
    let holdings = fetch_positions_impl(principal)
        .await
        .map_err(|e| format!("{:?}", e))?;
    cache::get().insert(principal, (holdings, now()));
    Ok(total)
}

#[cfg(all(feature = "claim", target_arch = "wasm32"))]
async fn claim_rewards_impl(principal: Principal) -> Result<u64, String> {
    use crate::cache;
    use ic_cdk::api::call::call;
    let factory_id = match crate::utils::env_principal("ICPSWAP_FACTORY") {
        Some(p) => p,
        None => return Err("factory".into()),
    };
    let ledger = crate::ledger_fetcher::LEDGERS
        .first()
        .cloned()
        .ok_or("ledger")?;
    let (pools,): (Vec<PoolData>,) = call(factory_id, "getPools", ()).await.map_err(|(_, e)| e)?;
    let mut total: u64 = 0;
    for pool in pools {
        let (spent,): (u64,) = call(pool.canister_id, "claim", (principal, ledger))
            .await
            .map_err(|(_, e)| e)?;
        total = total.checked_add(spent).ok_or("overflow")?;
    }
    let holdings = fetch_positions_impl(principal)
        .await
        .map_err(|e| format!("{:?}", e))?;
    cache::get().insert(principal, (holdings, now()));
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[tokio::test(flavor = "current_thread")]
    async fn fetch_positions_empty_without_env() {
        let adapter = IcpswapAdapter;
        let res = adapter.fetch_positions(Principal::anonymous()).await;
        assert!(matches!(res, Err(FetchError::InvalidConfig(_))));
    }

    #[cfg(feature = "claim")]
    #[tokio::test(flavor = "current_thread")]
    async fn claim_fails_without_env() {
        std::env::remove_var("ICPSWAP_FACTORY");
        let res = claim_rewards_impl(Principal::anonymous()).await;
        assert!(res.is_err());
    }

    #[quickcheck]
    fn fuzz_decode_pool(data: Vec<u8>) -> bool {
        let _ = Decode!(&data, Vec<PoolData>);
        true
    }
}
