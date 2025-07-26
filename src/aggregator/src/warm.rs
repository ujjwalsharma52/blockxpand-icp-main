use candid::Principal;
use once_cell::sync::Lazy;
use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;
use tracing::{debug, info};

struct Entry {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    cid: Principal,
    next: u64,
}

static MAX_QUEUE_SIZE: Lazy<usize> = Lazy::new(|| {
    option_env!("WARM_QUEUE_SIZE")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(128)
});

static QUEUE: Lazy<Mutex<VecDeque<Entry>>> =
    Lazy::new(|| Mutex::new(VecDeque::with_capacity(*MAX_QUEUE_SIZE)));

const ITEMS_PER_TICK: usize = 3;

pub fn init() {
    let now = crate::utils::now();
    let mut q = QUEUE.lock().unwrap();
    q.clear();
    let mut seen = HashSet::with_capacity(*MAX_QUEUE_SIZE);
    for cid in crate::ledger_fetcher::LEDGERS.iter().cloned() {
        if q.len() >= *MAX_QUEUE_SIZE {
            break;
        }
        if seen.insert(cid) {
            q.push_back(Entry { cid, next: now });
        }
    }
    for cid in crate::utils::dex_ids() {
        if q.len() >= *MAX_QUEUE_SIZE {
            break;
        }
        if seen.insert(cid) {
            q.push_back(Entry { cid, next: now });
        }
    }
    info!(queued = q.len(), "warm queue initialised");
}

pub async fn tick() {
    for _ in 0..ITEMS_PER_TICK {
        let entry_opt = {
            let mut q = QUEUE.lock().unwrap();
            q.pop_front()
        };
        let mut entry = match entry_opt {
            Some(e) => e,
            None => break,
        };

        if crate::utils::now() >= entry.next {
            #[cfg(not(target_arch = "wasm32"))]
            crate::ledger_fetcher::warm_metadata(entry.cid).await;
            crate::utils::warm_icrc_metadata(entry.cid).await;
            debug!("warmed metadata for {}", entry.cid);
            entry.next = crate::utils::now() + crate::utils::DAY_NS;
        }

        {
            let mut q = QUEUE.lock().unwrap();
            q.push_back(entry);
        }
    }
}

#[cfg(test)]
pub fn len() -> usize {
    QUEUE.lock().unwrap().len()
}

#[cfg(test)]
pub fn dump() -> Vec<Principal> {
    QUEUE.lock().unwrap().iter().map(|e| e.cid).collect()
}

#[cfg(test)]
pub fn init_for_tests(ledgers: Vec<Principal>, dexes: Vec<Principal>) {
    let now = crate::utils::now();
    let mut q = QUEUE.lock().unwrap();
    q.clear();
    let mut seen = HashSet::with_capacity(*MAX_QUEUE_SIZE);
    for cid in ledgers {
        if q.len() >= *MAX_QUEUE_SIZE {
            break;
        }
        if seen.insert(cid) {
            q.push_back(Entry { cid, next: now });
        }
    }
    for cid in dexes {
        if q.len() >= *MAX_QUEUE_SIZE {
            break;
        }
        if seen.insert(cid) {
            q.push_back(Entry { cid, next: now });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn gen_principal(i: u8) -> Principal {
        let bytes = [i; 32];
        Principal::self_authenticating(&bytes)
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial]
    async fn init_bounds_queue() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "[ledgers]").unwrap();
        for i in 0..150u8 {
            writeln!(f, "L{i} = \"{}\"", gen_principal(i).to_text()).unwrap();
        }
        writeln!(f, "[dex]").unwrap();
        for i in 0..150u8 {
            writeln!(f, "D{i} = \"{}\"", gen_principal(i).to_text()).unwrap();
        }
        std::env::set_var("LEDGERS_FILE", f.path());
        crate::utils::load_dex_config().await;
        let ledgers: Vec<Principal> = (0..150u8).map(gen_principal).collect();
        let dexes: Vec<Principal> = (0..150u8).map(gen_principal).collect();
        init_for_tests(ledgers, dexes);
        assert_eq!(len(), *MAX_QUEUE_SIZE);
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial]
    async fn init_deduplicates() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "[ledgers]\nA = \"aaaaa-aa\"\nB = \"aaaaa-aa\"").unwrap();
        writeln!(f, "[dex]\nX = \"aaaaa-aa\"\nY = \"aaaaa-aa\"").unwrap();
        std::env::set_var("LEDGERS_FILE", f.path());
        crate::utils::load_dex_config().await;
        let ledgers = vec![
            Principal::from_text("aaaaa-aa").unwrap(),
            Principal::from_text("aaaaa-aa").unwrap(),
        ];
        let dexes = vec![
            Principal::from_text("aaaaa-aa").unwrap(),
            Principal::from_text("aaaaa-aa").unwrap(),
        ];
        init_for_tests(ledgers, dexes);
        assert_eq!(len(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    #[serial]
    async fn deterministic_after_reinit() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "[ledgers]\nMOCK = \"aaaaa-aa\"").unwrap();
        writeln!(f, "[dex]\nX = \"aaaaa-aa\"").unwrap();
        std::env::set_var("LEDGERS_FILE", f.path());
        crate::utils::load_dex_config().await;
        let ledgers = vec![gen_principal(1)];
        let dexes = vec![gen_principal(2)];
        init_for_tests(ledgers, dexes);
        let first = dump();
        init_for_tests(vec![gen_principal(1)], vec![gen_principal(2)]);
        let second = dump();
        assert_eq!(first, second);
    }
}
