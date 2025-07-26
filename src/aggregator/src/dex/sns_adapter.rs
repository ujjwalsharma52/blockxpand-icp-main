use super::{DexAdapter, RewardInfo};
use crate::error::FetchError;
#[cfg(not(target_arch = "wasm32"))]
use crate::utils::{format_amount, get_agent};
use async_trait::async_trait;
use bx_core::Holding;
use candid::{CandidType, Nat, Principal};
#[cfg(not(target_arch = "wasm32"))]
use candid::{Decode, Encode};
#[cfg(not(target_arch = "wasm32"))]
use once_cell::sync::Lazy;
use serde::Deserialize;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;

pub struct SnsAdapter;

pub fn clear_cache() {}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::type_complexity)]
static MOCK_CLAIMABLE: Lazy<Mutex<Option<Result<Vec<Claimable>, String>>>> =
    Lazy::new(|| Mutex::new(None));
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::type_complexity)]
static MOCK_CLAIM: Lazy<Mutex<Option<Result<u64, String>>>> = Lazy::new(|| Mutex::new(None));

#[derive(CandidType, Deserialize, Clone)]
pub struct Claimable {
    pub symbol: String,
    pub amount: Nat,
    pub decimals: u8,
}

#[async_trait]
impl DexAdapter for SnsAdapter {
    async fn fetch_positions(&self, principal: Principal) -> Result<Vec<Holding>, FetchError> {
        fetch_positions_impl(principal).await
    }

    async fn claimable_rewards(&self, principal: Principal) -> Result<Vec<RewardInfo>, FetchError> {
        let holdings = fetch_positions_impl(principal).await?;
        Ok(holdings
            .into_iter()
            .map(|h| RewardInfo {
                token: h.token,
                amount: h.amount,
            })
            .collect())
    }

    #[cfg(feature = "claim")]
    async fn claim_rewards(&self, principal: Principal) -> Result<u64, String> {
        claim_impl(principal).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_positions_impl(principal: Principal) -> Result<Vec<Holding>, FetchError> {
    let distro_id = match crate::utils::env_principal("SNS_DISTRIBUTOR") {
        Some(p) => p,
        None => return Err(FetchError::InvalidConfig("distributor".into())),
    };
    if let Some(resp) = MOCK_CLAIMABLE.lock().unwrap().clone() {
        let claims = resp.map_err(FetchError::Network)?;
        return Ok(claims
            .into_iter()
            .map(|c| Holding {
                source: "SNS".into(),
                token: c.symbol,
                amount: format_amount(c.amount, c.decimals),
                status: "claimable".into(),
            })
            .collect());
    }
    let agent = get_agent().await;
    let claims = sns_get_claimable(&agent, distro_id, principal)
        .await
        .map_err(FetchError::from)?;
    let mut out = Vec::with_capacity(claims.len());
    for c in claims {
        out.push(Holding {
            source: "SNS".into(),
            token: c.symbol,
            amount: format_amount(c.amount, c.decimals),
            status: "claimable".into(),
        });
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
async fn fetch_positions_impl(_principal: Principal) -> Result<Vec<Holding>, FetchError> {
    Ok(Vec::new())
}

#[cfg(all(feature = "claim", not(target_arch = "wasm32")))]
async fn claim_impl(principal: Principal) -> Result<u64, String> {
    let distro_id = match crate::utils::env_principal("SNS_DISTRIBUTOR") {
        Some(p) => p,
        None => return Err("distributor".into()),
    };
    if let Some(resp) = MOCK_CLAIM.lock().unwrap().clone() {
        return resp;
    }
    let agent = get_agent().await;
    let spent = sns_claim(&agent, distro_id, principal)
        .await
        .map_err(|e| e.to_string())?;
    Ok(spent)
}

#[cfg(all(feature = "claim", target_arch = "wasm32"))]
async fn claim_impl(principal: Principal) -> Result<u64, String> {
    use ic_cdk::api::call::call;
    let distro_id = crate::utils::env_principal("SNS_DISTRIBUTOR").ok_or("distributor")?;
    let (spent,): (u64,) = call(distro_id, "claim", (principal,))
        .await
        .map_err(|(_, e)| e)?;
    Ok(spent)
}

#[cfg(not(target_arch = "wasm32"))]
pub mod test_helpers {
    use super::{Claimable, MOCK_CLAIM, MOCK_CLAIMABLE};

    /// Provide a mocked response for `get_claimable_tokens`.
    pub fn set_claimable(resp: Result<Vec<Claimable>, String>) {
        *MOCK_CLAIMABLE.lock().unwrap() = Some(resp);
    }

    /// Provide a mocked response for `claim`.
    pub fn set_claim(resp: Result<u64, String>) {
        *MOCK_CLAIM.lock().unwrap() = Some(resp);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sns_get_claimable(
    agent: &ic_agent::Agent,
    distro: Principal,
    principal: Principal,
) -> Result<Vec<Claimable>, ic_agent::AgentError> {
    if let Some(resp) = MOCK_CLAIMABLE.lock().unwrap().clone() {
        return resp.map_err(ic_agent::AgentError::MessageError);
    }
    let arg = Encode!(&principal).map_err(|e| ic_agent::AgentError::MessageError(e.to_string()))?;
    let bytes = agent
        .query(&distro, "get_claimable_tokens")
        .with_arg(arg)
        .call()
        .await?;
    let claims: Vec<Claimable> = Decode!(&bytes, Vec<Claimable>)
        .map_err(|_| ic_agent::AgentError::MessageError("invalid response".into()))?;
    Ok(claims)
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn sns_claim(
    agent: &ic_agent::Agent,
    distro: Principal,
    principal: Principal,
) -> Result<u64, ic_agent::AgentError> {
    if let Some(resp) = MOCK_CLAIM.lock().unwrap().clone() {
        return resp.map_err(ic_agent::AgentError::MessageError);
    }
    let arg = Encode!(&principal).map_err(|e| ic_agent::AgentError::MessageError(e.to_string()))?;
    let bytes = agent
        .update(&distro, "claim")
        .with_arg(arg)
        .call_and_wait()
        .await?;
    Decode!(&bytes, u64).map_err(|_| ic_agent::AgentError::MessageError("invalid response".into()))
}
