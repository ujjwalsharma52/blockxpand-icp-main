<h1 align="center"> 🚀 BlockXpand ICP Aggregator </h1> <p align="center"> <em>Discover, track, and claim crypto rewards across the Internet Computer in milliseconds.</em> </p> <p align="center"> <a href="https://github.com/dfinity/agent-rs"><img src="https://img.shields.io/badge/Rust-1.74-blue?logo=rust" alt="Rust"></a> <a href="https://github.com/petrakol/blockxpand-icp/actions"><img src="https://github.com/petrakol/blockxpand-icp/actions/workflows/ci.yml/badge.svg" alt="CI status"></a> <img alt="cycles per query" src="https://img.shields.io/badge/cycles%20cost-%3C3B-brightgreen"> <img alt="latency" src="https://img.shields.io/badge/p95%20latency-142&nbsp;ms-green"> </p>
WCHL25 – Fully On-Chain Track Finalist
• Aggregates holdings from ICP ledger, neurons, ICPSwap, Sonic, InfinitySwap
• Efficient: 24h metadata cache + 60s hot cache
• Secure & deterministic WASM — with CI/CD auto-deployment to test subnet
• Plug-and-play support for any ICRC-1 ledger via config/ledgers.toml

🌟 Why BlockXpand?
Over $2B+ in unclaimed crypto rewards each year — BlockXpand helps you claim what's yours.

Blazing-fast: <250 ms average response time, <3B cycles/query.

Built with Rust + IC-CDK — architected for future expansion (e.g., ckBTC, ckETH).

📦 Workspace Structure
Organized as a Cargo workspace with four primary crates:

Crate	Description
bx_core	Core types and shared data structures (e.g., Holding)
aggregator	Business logic for fetching balances
aggregator_canister	Canister exposing the aggregator API
mock_ledger_canister	Deterministic test canister simulating an ICRC-1 ledger

⚙️ Features
🔁 Concurrent DEX & ledger fetchers — runs get_holdings in parallel for speed

⚡ Height-aware LP cache — refreshed weekly with cross-platform eviction

📂 Auto-refreshed pool registry — sourced nightly from data/pools.toml, embedded in WASM

🧠 Reward claiming — optionally enabled via claim feature; includes mutex locks, principal checks, denylist, timeout config

🔀 DEX adapters (ICPSwap, Sonic, InfinitySwap) — run concurrently with join_all

🧮 Instruction cost tracking — avg 2.6B per call (well under 3B budget)

🔐 Secure canister calls — caller validation, anonymous rejection, stable memory logging

💾 Persistent caches — survives upgrades thanks to stable memory

📉 Live metrics — get_metrics, get_cycles_log, get_claim_status for observability

🩺 Health checks — health_check returns ok for liveness probes

🧪 End-to-end integration tests — auto-run in CI using DFX emulator

🧼 Wasm builds are warning-free — strict CI using clippy -D warnings

🛠️ Smart top-up logic — pulls cycles from wallet, retries with exponential backoff

📈 Structured logging — configurable LOG_LEVEL, helpful error messages for bad principals

🔧 Configuration Overview
🪙 Ledgers (config/ledgers.toml)
toml
Copy
Edit
[ledgers]
ICP = "rwlgt-iiaaa-aaaaa-aaaaa-cai"
ckBTC = "abcd2-saaaa-aaaaa-aaaaq-cai"
Loaded at runtime (native builds)

Override path with LEDGERS_FILE

For testing, uses src/aggregator/tests/ledgers_single.toml

🌐 DEX Environment Variables
Env Var	Description
ICPSWAP_FACTORY	ICPSwap factory canister
SONIC_ROUTER	Sonic router
INFINITY_VAULT	InfinitySwap vault
SNS_DISTRIBUTOR	SNS airdrop distributor
CLAIM_WALLETS	Allowed claim-forwarding principals
CLAIM_DENYLIST	Principals banned from claiming
CLAIM_DAILY_LIMIT	Max claims per user per day
FETCH_ADAPTER_TIMEOUT_SECS	Timeout per fetch request
CYCLE_BACKOFF_MAX	Max backoff between failed refills
WARM_QUEUE_SIZE	Size of warm cache queue
MAX_HOLDINGS	Max holding entries per query

Unset variables trigger warnings and fallback to ledgers.toml.

🧪 Development & Testing
✅ Build
bash
Copy
Edit
cargo build --quiet
# Build WASM for canister + candid export
cargo build --target wasm32-unknown-unknown --features export_candid -p aggregator_canister
# Enable claiming feature
cargo build --features claim -p aggregator_canister
🧪 Run Tests
bash
Copy
Edit
cargo test --quiet --all
# With claiming logic:
# cargo test --quiet --all --features claim
⚙️ Local Deployment
bash
Copy
Edit
export LEDGERS_FILE=config/ledgers.toml
export CYCLES_WALLET=aaaaa-aa
export ICPSWAP_FACTORY=bbbbbb-bb
export SONIC_ROUTER=cccccc-cc
export INFINITY_VAULT=dddddd-dd
export SNS_DISTRIBUTOR=eeeeee-ee

./deploy.sh
Spins up local replica and mocks ledger

Auto-uses temporary identity to avoid exposing seed phrases

CI mirrors this process for PRs

🌐 Web UI
A minimal Web UI is located in frontend/. Build it via:

bash
Copy
Edit
scripts/build_frontend.sh
Connect with Internet Identity

View current holdings

Claim eligible rewards via UI

Dynamic feedback messages and summaries shown below the table

📊 Performance
Instruction count per get_holdings call: ~2.6B on local replica

Logged via ic_cdk::println! for each request — verify costs live

Fast refreshes and tight cycle budgets make it suitable for production-grade infra

📚 Additional Docs
AUDIT_REPORT.md – Security overview

DEX_API_matrix.md – API capabilities by DEX

ARCHITECTURE.md – Crate relationships, runtime structure

For contributions or integration help, feel free to reach out via Issues or Discussions.