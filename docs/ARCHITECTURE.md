# Architecture Overview

This repository is a Cargo workspace composed of several crates that work together to fetch balances from multiple ledgers and DEXes and expose them through an Internet Computer canister.

## Crates

- **bx_core** – Shared data structures such as the `Holding` type.
- **aggregator** – Library containing all runtime logic:
  - ledger and neuron fetchers
  - DEX adapters
  - cycle top‑ups and heartbeat driven warm queue
  - LP cache and operational metrics
- **aggregator_canister** – Thin wrapper that exposes `aggregator` as a canister. It wires up init, heartbeat and upgrade hooks and optionally exports the Candid interface.
- **mock_*_canister** – Deterministic mock canisters used in unit and integration tests.

## Processes

1. **Warm queue** – On init the queue loads ledger and DEX IDs and gradually warms their metadata. The queue is bounded and deduplicates entries to avoid unbounded growth.
2. **Cycle monitor** – Every heartbeat checks the cycle balance and calls a wallet canister to top up when needed. Failures trigger exponential backoff and each event is logged in stable memory.
3. **Metrics** – Query and heartbeat counts plus cycle balance are tracked and can be queried via the `get_metrics` endpoint. Metrics state is preserved across upgrades.
4. **Upgrade flow** – Before upgrades the cycle log, ledger metadata and LP caches and metrics are saved to stable memory. They are restored in `post_upgrade` so the canister resumes operation without warming up again.

The [README](../README.md) explains how to configure environment variables and run the deployment script. The integration tests under `tests/` launch a local replica to exercise these processes end‑to‑end.

## Dependencies

Core crates are pinned through the workspace to ensure reproducible builds. Notable dependencies include:

- `ic-cdk` and `ic-cdk-macros` – canister interfaces and macros
- `ic-agent` – used off-chain to query ledgers during tests
- `tokio` – async runtime on native targets
- `tracing` / `tracing-subscriber` – structured logging with configurable levels
- `dashmap`, `notify` and `ic-certified-map` – concurrency, file watching and stable state helpers

Each crate depends on only the libraries it needs. For example the `aggregator` library uses `dashmap` for its cache and `serde_json` solely for stable storage.

## Data flow

1. A caller invokes `get_holdings` over Candid from the website or CLI.
2. The aggregator fetches balances from the ICP ledger, neurons and all configured DEXes concurrently.
3. Results are cached for 60 s with a certificate so repeat queries are cheap.
4. A heartbeat warms metadata and tops up cycles when required. Failures increment a backoff counter.
5. When built with the `claim` feature, `claim_all_rewards` verifies the caller and forwards claim calls to each DEX.

## Future improvements

The current architecture is functional but several enhancements would improve usability:

1. **Front-end integration** – Build a lightweight Web UI that calls the canister via Candid or HTTP, letting users connect their wallet and trigger `claim_all_rewards`.
2. **Persistent settings** – Store user preferences (e.g., favourite ledgers) in stable memory for a personalised experience.
3. **Expanded metrics** – Export cycle usage per query and reward-claim statistics to aid monitoring.
4. **Additional adapters** – Support upcoming DEXes or SNS token distributions so users can claim rewards from more sources. The generic `SnsAdapter` illustrates how new reward sources plug into the fetcher pipeline.

