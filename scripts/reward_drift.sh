#!/bin/bash
set -e
LOG=drift.log
echo "Starting reward drift check" > $LOG

dfx start --background --clean
trap 'dfx stop' EXIT

# deploy mock canisters
CANISTERS=(mock_ledger mock_icpswap mock_sonic mock_infinity aggregator)
for c in "${CANISTERS[@]}"; do
  dfx deploy "$c" >/dev/null
  id=$(dfx canister id "$c")
  echo "$c $id" >> $LOG
  case $c in
    mock_icpswap) export ICPSWAP_FACTORY=$id;;
    mock_sonic) export SONIC_ROUTER=$id;;
    mock_infinity) export INFINITY_VAULT=$id;;
    mock_ledger) export LEDGER_ID=$id;;
  esac
done

export LEDGER_URL="http://127.0.0.1:4943"

echo "Running drift binary" >> $LOG
cargo run --quiet --bin reward_drift >> $LOG 2>&1
