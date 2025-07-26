#[cfg(target_arch = "wasm32")]
use candid::Principal;
#[cfg(target_arch = "wasm32")]
use ic_cdk::api::{call::call, canister_balance128, time};
#[cfg(target_arch = "wasm32")]
use once_cell::sync::Lazy;
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static LAST_CHECK: RefCell<u64> = RefCell::new(0);
    static BACKOFF_UNTIL: RefCell<u64> = RefCell::new(0);
    static FAILURES: RefCell<u8> = RefCell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

#[cfg(target_arch = "wasm32")]
static WALLET: Lazy<Option<Principal>> =
    Lazy::new(|| option_env!("CYCLES_WALLET").and_then(|s| Principal::from_text(s).ok()));

#[cfg(target_arch = "wasm32")]
const MIN_BALANCE: u128 = 500_000_000_000; // 0.5 T

#[cfg(target_arch = "wasm32")]
const LOG_LIMIT: usize = 100;

#[cfg(target_arch = "wasm32")]
fn push_log(entry: String) {
    LOG.with(|l| {
        let mut log = l.borrow_mut();
        if log.len() >= LOG_LIMIT {
            log.remove(0);
        }
        log.push(entry);
    });
}

#[cfg(target_arch = "wasm32")]
fn max_backoff_minutes() -> u64 {
    option_env!("CYCLE_BACKOFF_MAX")
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
}

#[cfg(any(target_arch = "wasm32", test))]
fn compute_backoff_minutes(fails: u8, max: u64) -> u64 {
    (1u64 << fails.min(6) as u64).min(max.max(1))
}

#[cfg(target_arch = "wasm32")]
pub async fn tick() {
    use crate::utils::MINUTE_NS;
    let now = time();
    let allowed = BACKOFF_UNTIL.with(|b| now >= *b.borrow());
    if !allowed {
        tracing::debug!("cycle refill backoff active");
        return;
    }
    let run = LAST_CHECK.with(|c| {
        if now - *c.borrow() >= MINUTE_NS {
            *c.borrow_mut() = now;
            true
        } else {
            false
        }
    });
    if !run {
        return;
    }
    if canister_balance128() < MIN_BALANCE {
        tracing::debug!("balance below threshold, attempting refill");
        if let Some(w) = *WALLET {
            crate::metrics::inc_cycle_refill_attempt();
            let before = canister_balance128();
            let res: Result<(), _> = call(w, "wallet_receive", ()).await;
            let after = canister_balance128();
            if res.is_ok() && after > before {
                crate::metrics::inc_cycle_refill_success();
                FAILURES.with(|f| *f.borrow_mut() = 0);
                BACKOFF_UNTIL.with(|b| *b.borrow_mut() = now);
                push_log(format!("{now}: refilled to {after}"));
                tracing::info!("cycles refilled to {after}");
            } else {
                let fails = FAILURES.with(|f| {
                    let mut v = f.borrow_mut();
                    *v = v.saturating_add(1);
                    *v
                });
                let backoff_m = compute_backoff_minutes(fails, max_backoff_minutes());
                BACKOFF_UNTIL.with(|b| *b.borrow_mut() = now + backoff_m * MINUTE_NS);
                push_log(format!("{now}: refill failed, backoff {backoff_m}m"));
                tracing::warn!("cycles refill failed, backoff {backoff_m}m");
            }
        } else {
            tracing::warn!("CYCLES_WALLET not configured");
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn log() -> Vec<String> {
    LOG.with(|l| l.borrow().clone())
}

#[cfg(target_arch = "wasm32")]
pub fn take_log() -> Vec<String> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

#[cfg(target_arch = "wasm32")]
pub fn set_log(log: Vec<String>) {
    LOG.with(|l| {
        let mut target = l.borrow_mut();
        *target = if log.len() > LOG_LIMIT {
            log[log.len() - LOG_LIMIT..].to_vec()
        } else {
            log
        };
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn tick() {}
#[cfg(not(target_arch = "wasm32"))]
pub fn log() -> Vec<String> {
    Vec::new()
}
#[cfg(not(target_arch = "wasm32"))]
pub fn take_log() -> Vec<String> {
    Vec::new()
}
#[cfg(not(target_arch = "wasm32"))]
pub fn set_log(_: Vec<String>) {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::compute_backoff_minutes;

    #[test]
    fn backoff_growth_and_cap() {
        assert_eq!(compute_backoff_minutes(0, 60), 1);
        assert_eq!(compute_backoff_minutes(1, 60), 2);
        assert_eq!(compute_backoff_minutes(5, 60), 32);
        assert_eq!(compute_backoff_minutes(7, 60), 60);
    }
}
