use candid::CandidType;
use core::sync::atomic::{AtomicU64, Ordering};
use serde::Serialize;

static QUERY_COUNT: AtomicU64 = AtomicU64::new(0);
static HEARTBEAT_COUNT: AtomicU64 = AtomicU64::new(0);
static LAST_HEARTBEAT: AtomicU64 = AtomicU64::new(0);
static CLAIM_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static CLAIM_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static CYCLE_REFILL_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static CYCLE_REFILL_SUCCESSES: AtomicU64 = AtomicU64::new(0);

#[derive(CandidType, Serialize)]
pub struct Metrics {
    pub cycles: u128,
    pub query_count: u64,
    pub heartbeat_count: u64,
    pub last_heartbeat: u64,
    pub claim_attempts: u64,
    pub claim_successes: u64,
    pub cycle_refill_attempts: u64,
    pub cycle_refill_successes: u64,
}

pub fn inc_query() {
    QUERY_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_claim_attempt() {
    CLAIM_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_claim_success() {
    CLAIM_SUCCESSES.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_cycle_refill_attempt() {
    CYCLE_REFILL_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_cycle_refill_success() {
    CYCLE_REFILL_SUCCESSES.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_heartbeat(now: u64) {
    HEARTBEAT_COUNT.fetch_add(1, Ordering::Relaxed);
    LAST_HEARTBEAT.store(now, Ordering::Relaxed);
}

pub fn get() -> Metrics {
    let cycles = if cfg!(target_arch = "wasm32") {
        ic_cdk::api::canister_balance128()
    } else {
        0
    };
    Metrics {
        cycles,
        query_count: QUERY_COUNT.load(Ordering::Relaxed),
        heartbeat_count: HEARTBEAT_COUNT.load(Ordering::Relaxed),
        last_heartbeat: LAST_HEARTBEAT.load(Ordering::Relaxed),
        claim_attempts: CLAIM_ATTEMPTS.load(Ordering::Relaxed),
        claim_successes: CLAIM_SUCCESSES.load(Ordering::Relaxed),
        cycle_refill_attempts: CYCLE_REFILL_ATTEMPTS.load(Ordering::Relaxed),
        cycle_refill_successes: CYCLE_REFILL_SUCCESSES.load(Ordering::Relaxed),
    }
}

#[cfg(target_arch = "wasm32")]
pub fn stable_save() -> (u64, u64, u64, u64, u64, u64, u64) {
    (
        QUERY_COUNT.load(Ordering::Relaxed),
        HEARTBEAT_COUNT.load(Ordering::Relaxed),
        LAST_HEARTBEAT.load(Ordering::Relaxed),
        CLAIM_ATTEMPTS.load(Ordering::Relaxed),
        CLAIM_SUCCESSES.load(Ordering::Relaxed),
        CYCLE_REFILL_ATTEMPTS.load(Ordering::Relaxed),
        CYCLE_REFILL_SUCCESSES.load(Ordering::Relaxed),
    )
}

#[cfg(target_arch = "wasm32")]
pub fn stable_restore(data: (u64, u64, u64, u64, u64, u64, u64)) {
    QUERY_COUNT.store(data.0, Ordering::Relaxed);
    HEARTBEAT_COUNT.store(data.1, Ordering::Relaxed);
    LAST_HEARTBEAT.store(data.2, Ordering::Relaxed);
    CLAIM_ATTEMPTS.store(data.3, Ordering::Relaxed);
    CLAIM_SUCCESSES.store(data.4, Ordering::Relaxed);
    CYCLE_REFILL_ATTEMPTS.store(data.5, Ordering::Relaxed);
    CYCLE_REFILL_SUCCESSES.store(data.6, Ordering::Relaxed);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stable_save() -> (u64, u64, u64, u64, u64, u64, u64) {
    (0, 0, 0, 0, 0, 0, 0)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn stable_restore(_: (u64, u64, u64, u64, u64, u64, u64)) {}
