use super::DexAdapter;
use crate::error::FetchError;
#[cfg(all(feature = "claim", not(target_arch = "wasm32")))]
use crate::utils::now;
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    lp_cache,
    utils::{format_amount, get_agent},
};
use async_trait::async_trait;
use bx_core::Holding;
use candid::{CandidType, Nat, Principal};
#[cfg(not(target_arch = "wasm32"))]
use candid::{Decode, Encode};
use serde::Deserialize;

#[derive(CandidType, Deserialize, Clone)]
struct Token {
    address: String,
    decimals: u8,
}

#[derive(CandidType, Deserialize, Clone)]
struct PositionInfo {
    token_a: Token,
    token_b: Token,
    #[serde(rename = "token_a_amount")]
    token_a_amount: Nat,
    #[serde(rename = "token_b_amount")]
    token_b_amount: Nat,
    reward_token: Token,
    reward_amount: Nat,
    auto_compound: bool,
}

pub struct SonicAdapter;

#[cfg(not(target_arch = "wasm32"))]
pub fn clear_cache() {}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_positions_impl(principal: Principal) -> Result<Vec<Holding>, FetchError> {
    let router_id = match crate::utils::env_principal("SONIC_ROUTER") {
        Some(p) => p,
        None => return Err(FetchError::InvalidConfig("router".into())),
    };
    let agent = get_agent().await;
    let arg = Encode!(&principal).map_err(|_| FetchError::InvalidResponse)?;
    let bytes = match agent
        .query(&router_id, "get_user_positions")
        .with_arg(arg)
        .call()
        .await
    {
        Ok(b) => b,
        Err(e) => return Err(FetchError::from(e)),
    };
    let positions: Vec<PositionInfo> =
        Decode!(&bytes, Vec<PositionInfo>).map_err(|_| FetchError::InvalidResponse)?;
    let height = crate::utils::dex_block_height(&agent, router_id)
        .await
        .unwrap_or(0);
    let holdings = lp_cache::get_or_fetch(principal, "sonic", height, || async {
        let mut temp = Vec::with_capacity(positions.len() * 3);
        for pos in positions {
            let a0 = format_amount(pos.token_a_amount, pos.token_a.decimals);
            temp.push(Holding {
                source: "Sonic".into(),
                token: pos.token_a.address.clone(),
                amount: a0,
                status: "lp_escrow".into(),
            });
            let a1 = format_amount(pos.token_b_amount, pos.token_b.decimals);
            temp.push(Holding {
                source: "Sonic".into(),
                token: pos.token_b.address.clone(),
                amount: a1,
                status: "lp_escrow".into(),
            });
            if !pos.auto_compound {
                let ra = format_amount(pos.reward_amount, pos.reward_token.decimals);
                temp.push(Holding {
                    source: "Sonic".into(),
                    token: pos.reward_token.address.clone(),
                    amount: ra,
                    status: "lp_escrow".into(),
                });
            }
        }
        temp
    })
    .await;
    Ok(holdings)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_positions_impl(_principal: Principal) -> Result<Vec<Holding>, FetchError> {
    Ok(Vec::new())
}

#[cfg(all(feature = "claim", not(target_arch = "wasm32")))]
async fn claim_impl(principal: Principal) -> Result<u64, String> {
    use crate::{cache, ledger_fetcher::LEDGERS};
    let router_id = match crate::utils::env_principal("SONIC_ROUTER") {
        Some(p) => p,
        None => return Err("router".into()),
    };
    let ledger = LEDGERS.first().cloned().ok_or("ledger")?;
    let agent = get_agent().await;
    let arg = Encode!(&principal, &ledger).map_err(|e| e.to_string())?;
    let bytes = agent
        .update(&router_id, "claim")
        .with_arg(arg)
        .call_and_wait()
        .await
        .map_err(|e| e.to_string())?;
    let spent: u64 = Decode!(&bytes, u64).map_err(|_| "invalid response")?;
    let holdings = fetch_positions_impl(principal)
        .await
        .map_err(|e| format!("{:?}", e))?;
    cache::get().insert(principal, (holdings, now()));
    Ok(spent)
}

#[async_trait]
impl DexAdapter for SonicAdapter {
    async fn fetch_positions(&self, principal: Principal) -> Result<Vec<Holding>, FetchError> {
        fetch_positions_impl(principal).await
    }

    #[cfg(feature = "claim")]
    async fn claim_rewards(&self, principal: Principal) -> Result<u64, String> {
        claim_impl(principal).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use quickcheck_macros::quickcheck;

    #[tokio::test]
    async fn empty_without_env() {
        std::env::remove_var("SONIC_ROUTER");
        let adapter = SonicAdapter;
        let res = adapter.fetch_positions(Principal::anonymous()).await;
        assert!(matches!(res, Err(FetchError::InvalidConfig(_))));
    }

    #[quickcheck]
    fn fuzz_decode_position(data: Vec<u8>) -> bool {
        let _ = Decode!(&data, Vec<PositionInfo>);
        true
    }
}
