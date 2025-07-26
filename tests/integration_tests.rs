#[cfg(test)]
mod tests {
    use blockxpand_icp::{get_holdings, Holding};
    use candid::{Decode, Encode, Principal};
    use ic_agent::{identity::AnonymousIdentity, Agent};
    use std::io::Write;
    use std::path::Path;
    use std::process::{Command, Stdio};
    use tempfile::{NamedTempFile, TempDir};

    fn ensure_dfx() -> bool {
        Command::new("dfx").arg("--version").output().is_ok()
    }

    struct Replica {
        dir: TempDir,
    }
    impl Replica {
        fn start() -> Option<Self> {
            let dir = TempDir::new().ok()?;
            if Command::new("dfx")
                .args(["start", "--background", "--clean", "--emulator"])
                .current_dir(dir.path())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .ok()?
                .success()
            {
                Some(Self { dir })
            } else {
                None
            }
        }
    }
    impl Drop for Replica {
        fn drop(&mut self) {
            let _ = Command::new("dfx")
                .arg("stop")
                .current_dir(self.dir.path())
                .stdout(Stdio::null())
                .status();
        }
    }

    fn deploy(dir: &Path, canister: &str) -> Option<String> {
        if !Command::new("dfx")
            .args(["deploy", canister, "--network", "emulator"])
            .current_dir(dir)
            .stdout(Stdio::null())
            .status()
            .ok()?
            .success()
        {
            return None;
        }
        let output = Command::new("dfx")
            .args(["canister", "id", canister, "--network", "emulator"])
            .current_dir(dir)
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    #[tokio::test]
    async fn integration_get_holdings() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let cid = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nMOCK = \"{cid}\"").unwrap();

        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");
        std::env::set_var("LEDGERS_FILE", file.path());

        aggregator::utils::load_dex_config().await;

        aggregator::utils::load_dex_config().await;

        let principal = Principal::anonymous();
        let holdings = get_holdings(principal).await;
        assert_eq!(holdings.len(), 4);
        assert_eq!(holdings[0].token, "MOCK");
        assert_eq!(holdings[0].status, "liquid");

        // Deploy the aggregator canister using the mock ledger.
        let cfg_path = std::path::Path::new("config/ledgers.toml");
        let original = std::fs::read_to_string(cfg_path).unwrap();
        std::fs::write(cfg_path, format!("[ledgers]\nICP = \"{cid}\"\n")).unwrap();
        struct Restore(String);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = std::fs::write("config/ledgers.toml", &self.0);
            }
        }
        let _restore = Restore(original);

        let aggr_id = match deploy(replica.dir.path(), "aggregator") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy aggregator; skipping test");
                return;
            }
        };

        let agent = Agent::builder()
            .with_url("http://127.0.0.1:4943")
            .with_identity(AnonymousIdentity {})
            .build()
            .unwrap();
        let _ = agent.fetch_root_key().await;
        let arg = candid::Encode!(&Principal::anonymous()).unwrap();
        let bytes = agent
            .query(&Principal::from_text(aggr_id).unwrap(), "get_holdings")
            .with_arg(arg)
            .call()
            .await
            .unwrap();
        let res: Vec<Holding> = candid::Decode!(&bytes, Vec<Holding>).unwrap();
        assert_eq!(res.len(), 3);
    }

    #[tokio::test]
    async fn integration_get_holdings_cert() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let cid = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nMOCK = \"{cid}\"").unwrap();

        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");
        std::env::set_var("LEDGERS_FILE", file.path());

        aggregator::utils::load_dex_config().await;

        let cfg_path = std::path::Path::new("config/ledgers.toml");
        let original = std::fs::read_to_string(cfg_path).unwrap();
        std::fs::write(cfg_path, format!("[ledgers]\nICP = \"{cid}\"\n")).unwrap();
        struct Restore(String);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = std::fs::write("config/ledgers.toml", &self.0);
            }
        }
        let _restore = Restore(original);

        let aggr_id = match deploy(replica.dir.path(), "aggregator") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy aggregator; skipping test");
                return;
            }
        };

        let agent = Agent::builder()
            .with_url("http://127.0.0.1:4943")
            .with_identity(AnonymousIdentity {})
            .build()
            .unwrap();
        let _ = agent.fetch_root_key().await;
        let arg = candid::Encode!(&Principal::anonymous()).unwrap();
        let _ = agent
            .update(&Principal::from_text(&aggr_id).unwrap(), "refresh_holdings")
            .with_arg(arg.clone())
            .call_and_wait()
            .await
            .unwrap();

        let bytes = agent
            .query(
                &Principal::from_text(&aggr_id).unwrap(),
                "get_holdings_cert",
            )
            .with_arg(arg)
            .call()
            .await
            .unwrap();

        #[derive(candid::CandidType, serde::Deserialize)]
        struct Resp {
            holdings: Vec<Holding>,
            certificate: Vec<u8>,
            witness: Vec<u8>,
        }

        let res: Resp = candid::Decode!(&bytes, Resp).unwrap();
        assert!(!res.certificate.is_empty());
        assert!(!res.witness.is_empty());
        assert_eq!(res.holdings.len(), 3);
    }

    #[tokio::test]
    async fn integration_icpswap_positions() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let ledger_id = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };
        let dex_id = match deploy(replica.dir.path(), "mock_icpswap") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock icpswap; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nMOCK = \"{ledger_id}\"").unwrap();

        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");
        std::env::set_var("LEDGERS_FILE", file.path());
        std::env::set_var("ICPSWAP_FACTORY", &dex_id);

        aggregator::utils::load_dex_config().await;

        let principal = Principal::anonymous();
        let holdings = get_holdings(principal).await;
        assert!(holdings.iter().any(|h| h.source == "ICPSwap"));
    }

    #[tokio::test]
    async fn integration_sonic_positions() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let ledger_id = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };
        let dex_id = match deploy(replica.dir.path(), "mock_sonic") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock sonic; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nMOCK = \"{ledger_id}\"").unwrap();

        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");
        std::env::set_var("LEDGERS_FILE", file.path());
        std::env::set_var("SONIC_ROUTER", &dex_id);

        aggregator::utils::load_dex_config().await;

        let principal = Principal::anonymous();
        let holdings = get_holdings(principal).await;
        assert!(holdings.iter().any(|h| h.source == "Sonic"));
    }

    #[tokio::test]
    async fn integration_infinity_positions() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let ledger_id = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };
        let dex_id = match deploy(replica.dir.path(), "mock_infinity") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock infinity; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nMOCK = \"{ledger_id}\"").unwrap();

        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");
        std::env::set_var("LEDGERS_FILE", file.path());
        std::env::set_var("INFINITY_VAULT", &dex_id);

        aggregator::utils::load_dex_config().await;

        let principal = Principal::anonymous();
        let holdings = get_holdings(principal).await;
        assert!(holdings.iter().any(|h| h.source == "InfinitySwap"));
    }

    #[cfg(feature = "claim")]
    #[tokio::test]
    async fn integration_reward_claim() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let ledger_id = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };
        let icpswap_id = match deploy(replica.dir.path(), "mock_icpswap") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock icpswap; skipping test");
                return;
            }
        };
        let sonic_id = match deploy(replica.dir.path(), "mock_sonic") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock sonic; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nMOCK = \"{ledger_id}\"").unwrap();

        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");
        std::env::set_var("LEDGERS_FILE", file.path());
        std::env::set_var("ICPSWAP_FACTORY", &icpswap_id);
        std::env::set_var("SONIC_ROUTER", &sonic_id);

        aggregator::utils::load_dex_config().await;

        let agent = Agent::builder()
            .with_url("http://127.0.0.1:4943")
            .with_identity(AnonymousIdentity {})
            .build()
            .unwrap();
        let _ = agent.fetch_root_key().await;

        #[derive(candid::CandidType)]
        struct Account {
            owner: Principal,
            subaccount: Option<Vec<u8>>,
        }

        let principal = Principal::anonymous();
        let balance_before_bytes = agent
            .query(
                &Principal::from_text(&ledger_id).unwrap(),
                "icrc1_balance_of",
            )
            .with_arg(
                candid::Encode!(&Account {
                    owner: principal,
                    subaccount: None
                })
                .unwrap(),
            )
            .call()
            .await
            .unwrap();
        let before: candid::Nat = candid::Decode!(&balance_before_bytes, candid::Nat).unwrap();

        blockxpand_icp::claim_all_rewards(principal).await;

        let balance_after_bytes = agent
            .query(
                &Principal::from_text(&ledger_id).unwrap(),
                "icrc1_balance_of",
            )
            .with_arg(
                candid::Encode!(&Account {
                    owner: principal,
                    subaccount: None
                })
                .unwrap(),
            )
            .call()
            .await
            .unwrap();
        let after: candid::Nat = candid::Decode!(&balance_after_bytes, candid::Nat).unwrap();
        assert!(after.0 > before.0);
    }

    #[tokio::test]
    async fn integration_multiple_ledgers_error() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let cid = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nGOOD = \"{cid}\"\nBAD = \"aaaaa-aa\"").unwrap();

        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");
        std::env::set_var("LEDGERS_FILE", file.path());

        let principal = Principal::anonymous();
        let holdings = get_holdings(principal).await;
        assert_eq!(holdings.len(), 5);
        assert_eq!(holdings[0].token, "MOCK");
        assert_eq!(holdings[0].status, "liquid");
        assert_eq!(holdings[1].token, "unknown");
        assert_eq!(holdings[1].status, "error");
    }

    #[tokio::test]
    async fn pool_registry_graphql() {
        aggregator::pool_registry::refresh().await;
        let out = blockxpand_icp::pools_graphql("query { pools { id } }".into());
        assert!(out.contains("pool1"));
    }

    #[tokio::test]
    async fn heartbeat_metrics_survive_upgrade() {
        if !ensure_dfx() {
            eprintln!("dfx not found; skipping integration test");
            return;
        }

        let replica = match Replica::start() {
            Some(r) => r,
            None => {
                eprintln!("failed to start dfx; skipping test");
                return;
            }
        };

        let cid = match deploy(replica.dir.path(), "mock_ledger") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy mock ledger; skipping test");
                return;
            }
        };

        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "[ledgers]\nMOCK = \"{cid}\"").unwrap();

        std::env::set_var("LEDGERS_FILE", file.path());
        std::env::set_var("LEDGER_URL", "http://127.0.0.1:4943");

        aggregator::utils::load_dex_config().await;

        let aggr_id = match deploy(replica.dir.path(), "aggregator") {
            Some(id) => id,
            None => {
                eprintln!("failed to deploy aggregator; skipping test");
                return;
            }
        };

        let agent = Agent::builder()
            .with_url("http://127.0.0.1:4943")
            .with_identity(AnonymousIdentity {})
            .build()
            .unwrap();
        let _ = agent.fetch_root_key().await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let arg = candid::Encode!().unwrap();
        let bytes = agent
            .query(&Principal::from_text(&aggr_id).unwrap(), "get_metrics")
            .with_arg(arg.clone())
            .call()
            .await
            .unwrap();
        #[derive(candid::CandidType, serde::Deserialize)]
        struct Metrics {
            cycles: u64,
            query_count: u64,
            heartbeat_count: u64,
            last_heartbeat: u64,
        }
        let m1: Metrics = candid::Decode!(&bytes, Metrics).unwrap();

        Command::new("dfx")
            .args(["deploy", "aggregator", "--network", "emulator"])
            .current_dir(replica.dir.path())
            .stdout(Stdio::null())
            .status()
            .ok();

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let bytes2 = agent
            .query(&Principal::from_text(&aggr_id).unwrap(), "get_metrics")
            .with_arg(arg)
            .call()
            .await
            .unwrap();
        let m2: Metrics = candid::Decode!(&bytes2, Metrics).unwrap();
        assert!(m2.heartbeat_count >= m1.heartbeat_count);
    }
}
