use candid::CandidType;
use once_cell::sync::Lazy;
#[cfg(not(target_arch = "wasm32"))]
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, CandidType, Serialize, Deserialize)]
pub struct PoolMeta {
    pub id: String,
    pub token_a: String,
    pub token_b: String,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub image_a: Option<String>,
    pub image_b: Option<String>,
}

#[derive(Deserialize)]
struct PoolsFile {
    pool: Vec<PoolMeta>,
}

static REGISTRY: Lazy<RwLock<HashMap<String, PoolMeta>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[cfg(not(target_arch = "wasm32"))]
static WATCHER: OnceCell<notify::RecommendedWatcher> = OnceCell::new();

pub fn list() -> Vec<PoolMeta> {
    REGISTRY.read().unwrap().values().cloned().collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn refresh() {
    let path = std::env::var("POOLS_FILE").unwrap_or_else(|_| "data/pools.toml".into());
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => load_content(&content),
        Err(e) => tracing::error!("pool registry refresh failed: {e}"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn watch_pools_file() {
    use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::path::Path;
    if WATCHER.get().is_some() {
        tracing::debug!("pools watcher already running");
        return;
    }
    let path = std::env::var("POOLS_FILE").unwrap_or_else(|_| "data/pools.toml".to_string());
    if !Path::new(&path).exists() {
        tracing::error!("pools file {path} missing");
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
        tracing::error!("failed to watch pools file: {e}");
        return;
    }
    let _ = WATCHER.set(watcher);
    tracing::info!("watching pools file at {}", path);
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            refresh().await;
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub async fn refresh() {
    load_content(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/pools.toml"
    )));
}

fn load_content(content: &str) {
    if let Ok(pf) = toml::from_str::<PoolsFile>(content) {
        let count = pf.pool.len();
        let mut map = HashMap::with_capacity(count);
        for p in pf.pool.into_iter() {
            map.insert(p.id.clone(), p);
        }
        *REGISTRY.write().unwrap() = map;
        tracing::info!(count, "pool registry loaded");
    }
}

#[cfg(target_arch = "wasm32")]
pub fn schedule_refresh() {
    use std::time::Duration;
    ic_cdk_timers::set_timer_interval(Duration::from_secs(crate::utils::DAY_SECS), || {
        ic_cdk::spawn(async { refresh().await });
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn schedule_refresh() {
    use std::time::Duration;
    tokio::spawn(async {
        let mut timer = tokio::time::interval(Duration::from_secs(crate::utils::DAY_SECS));
        loop {
            timer.tick().await;
            refresh().await;
        }
    });
}

pub fn graphql(_query: String) -> String {
    let data = list();
    serde_json::json!({"data": {"pools": data}}).to_string()
}
