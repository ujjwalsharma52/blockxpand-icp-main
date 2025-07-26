#!/bin/bash
set -e

# Example deployment step using dfx
# Starts a local replica and deploys the canister so CI can exercise the
# deployment process without needing access to a remote network.
# Create a throwaway identity so no mnemonic appears in the logs.
dfx identity new ci --force --storage-mode plaintext >/dev/null 2>&1 || true
dfx identity use ci
dfx start --background --clean
trap 'dfx stop' EXIT
cargo build --target wasm32-unknown-unknown --features export_candid -p aggregator_canister --quiet
dfx deploy
