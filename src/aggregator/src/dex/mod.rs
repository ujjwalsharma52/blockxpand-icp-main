use crate::error::FetchError;
use async_trait::async_trait;
use bx_core::Holding;
use candid::Principal;

#[derive(Debug, Clone, PartialEq)]
pub struct RewardInfo {
    pub token: String,
    pub amount: String,
}

#[async_trait]
pub trait DexAdapter: Send + Sync {
    async fn fetch_positions(&self, principal: Principal) -> Result<Vec<Holding>, FetchError>;
    async fn claimable_rewards(
        &self,
        _principal: Principal,
    ) -> Result<Vec<RewardInfo>, FetchError> {
        Ok(Vec::new())
    }
    #[cfg(feature = "claim")]
    async fn claim_rewards(&self, _principal: Principal) -> Result<u64, String> {
        Ok(0)
    }
}

pub mod dex_icpswap;
pub mod dex_infinity;
pub mod dex_sonic;
pub mod sns_adapter;

/// Clear cached metadata for all adapters
pub fn clear_all_caches() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        dex_icpswap::clear_cache();
        dex_sonic::clear_cache();
        dex_infinity::clear_cache();
    }
    sns_adapter::clear_cache();
}
