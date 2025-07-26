use aggregator::utils::{env_principal, get_agent};
use anyhow::Result;
use candid::CandidType;
use candid::{Decode, Encode, Nat, Principal};
use num_traits::{cast::ToPrimitive, Zero};
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
struct IcpswapPos {
    token0_amount: u64,
    token1_amount: u64,
}

#[derive(CandidType, Deserialize)]
struct SonicToken {
    address: String,
    decimals: u8,
}

#[derive(CandidType, Deserialize)]
struct SonicPos {
    token_a: SonicToken,
    token_b: SonicToken,
    token_a_amount: u64,
    token_b_amount: u64,
    reward_token: SonicToken,
    reward_amount: u64,
    auto_compound: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let agent = get_agent().await;
    let principal = Principal::anonymous();
    let mut failures = Vec::new();

    if let Some(cid) = env_principal("ICPSWAP_FACTORY") {
        let drift = check_icpswap(&agent, cid, principal).await?;
        println!("ICPSwap drift: {drift}");
        if drift > 0.01 {
            failures.push("ICPSwap");
        }
    }

    if let Some(cid) = env_principal("SONIC_ROUTER") {
        let drift = check_sonic(&agent, cid, principal).await?;
        println!("Sonic drift: {drift}");
        if drift > 0.01 {
            failures.push("Sonic");
        }
    }

    if let Some(cid) = env_principal("INFINITY_VAULT") {
        let drift = check_infinity(&agent, cid, principal).await?;
        println!("InfinitySwap drift: {drift}");
        if drift > 0.01 {
            failures.push("InfinitySwap");
        }
    }

    if !failures.is_empty() {
        eprintln!("Reward math drift detected: {failures:?}");
        std::process::exit(1);
    }
    Ok(())
}

async fn check_icpswap(agent: &ic_agent::Agent, cid: Principal, p: Principal) -> Result<f64> {
    let arg = Encode!(&p)?;
    let bytes = agent
        .query(&cid, "get_user_positions_by_principal")
        .with_arg(arg)
        .call()
        .await?;
    let positions: Vec<IcpswapPos> = Decode!(&bytes, Vec<IcpswapPos>)?;
    let share: u64 = positions
        .iter()
        .map(|x| x.token0_amount + x.token1_amount)
        .sum();
    let total_supply: Nat = Decode!(
        &agent
            .query(&cid, "lp_total_supply")
            .with_arg(Encode!()?)
            .call()
            .await?,
        Nat
    )?;
    let total_rewards: Nat = Decode!(
        &agent
            .query(&cid, "total_rewards")
            .with_arg(Encode!()?)
            .call()
            .await?,
        Nat
    )?;
    let claimable: Nat = Decode!(
        &agent
            .query(&cid, "claimable_rewards")
            .with_arg(Encode!(&p)?)
            .call()
            .await?,
        Nat
    )?;
    let predicted = total_rewards.0.to_u64().unwrap_or(0) as f64 * share as f64
        / total_supply.0.to_u64().unwrap_or(1) as f64;
    let actual = claimable.0.to_u64().unwrap_or(0) as f64;
    Ok(((actual - predicted).abs()) / predicted.max(1.0))
}

async fn check_sonic(agent: &ic_agent::Agent, cid: Principal, p: Principal) -> Result<f64> {
    let arg = Encode!(&p)?;
    let bytes = agent
        .query(&cid, "get_user_positions")
        .with_arg(arg)
        .call()
        .await?;
    let positions: Vec<SonicPos> = Decode!(&bytes, Vec<SonicPos>)?;
    let share: u64 = positions
        .iter()
        .map(|x| x.token_a_amount + x.token_b_amount)
        .sum();
    let total_supply: Nat = Decode!(
        &agent
            .query(&cid, "lp_total_supply")
            .with_arg(Encode!()?)
            .call()
            .await?,
        Nat
    )?;
    let total_rewards: Nat = Decode!(
        &agent
            .query(&cid, "total_rewards")
            .with_arg(Encode!()?)
            .call()
            .await?,
        Nat
    )?;
    let claimable: Nat = Decode!(
        &agent
            .query(&cid, "claimable_rewards")
            .with_arg(Encode!(&p)?)
            .call()
            .await?,
        Nat
    )?;
    let predicted = total_rewards.0.to_u64().unwrap_or(0) as f64 * share as f64
        / total_supply.0.to_u64().unwrap_or(1) as f64;
    let actual = claimable.0.to_u64().unwrap_or(0) as f64;
    Ok(((actual - predicted).abs()) / predicted.max(1.0))
}

async fn check_infinity(agent: &ic_agent::Agent, cid: Principal, p: Principal) -> Result<f64> {
    let total_rewards: Nat = Decode!(
        &agent
            .query(&cid, "total_rewards")
            .with_arg(Encode!()?)
            .call()
            .await?,
        Nat
    )?;
    if total_rewards.0.is_zero() {
        return Ok(0.0);
    }
    let claimable: Nat = Decode!(
        &agent
            .query(&cid, "claimable_rewards")
            .with_arg(Encode!(&p)?)
            .call()
            .await?,
        Nat
    )?;
    let predicted = 0f64;
    let actual = claimable.0.to_u64().unwrap_or(0) as f64;
    Ok(((actual - predicted).abs()) / 1f64)
}
