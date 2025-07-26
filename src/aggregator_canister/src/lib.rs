pub use aggregator::*;

#[ic_cdk_macros::init]
fn init() {
    aggregator::logging::init();
    #[cfg(not(target_arch = "wasm32"))]
    ic_cdk::spawn(async { aggregator::utils::load_dex_config().await });
    #[cfg(not(target_arch = "wasm32"))]
    aggregator::utils::watch_dex_config();
    #[cfg(not(target_arch = "wasm32"))]
    aggregator::pool_registry::watch_pools_file();
    ic_cdk::spawn(async { aggregator::pool_registry::refresh().await });
    aggregator::pool_registry::schedule_refresh();
    aggregator::lp_cache::schedule_eviction();
    aggregator::warm::init();
}

#[ic_cdk_macros::pre_upgrade]
fn pre_upgrade() {
    let log = aggregator::cycles::take_log();
    let meta = aggregator::ledger_fetcher::stable_save();
    let lp = aggregator::lp_cache::stable_save();
    let metrics = aggregator::metrics::stable_save();
    ic_cdk::storage::stable_save((log, meta, lp, metrics)).unwrap();
}

#[ic_cdk_macros::post_upgrade]
fn post_upgrade() {
    if let Ok((log, meta, lp, metrics)) = ic_cdk::storage::stable_restore::<(
        Vec<String>,
        Vec<aggregator::ledger_fetcher::StableMeta>,
        Vec<aggregator::lp_cache::StableEntry>,
        (u64, u64, u64, u64, u64, u64, u64),
    )>() {
        aggregator::cycles::set_log(log);
        aggregator::ledger_fetcher::stable_restore(meta);
        aggregator::lp_cache::stable_restore(lp);
        aggregator::metrics::stable_restore(metrics);
    }
}

#[ic_cdk_macros::heartbeat]
async fn heartbeat() {
    aggregator::metrics::inc_heartbeat(aggregator::utils::now());
    aggregator::cycles::tick().await;
    aggregator::warm::tick().await;
}

#[ic_cdk_macros::query]
fn get_metrics() -> aggregator::metrics::Metrics {
    aggregator::metrics::get()
}
#[cfg(feature = "export_candid")]
ic_cdk::export_candid!();
