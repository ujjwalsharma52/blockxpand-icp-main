#!/bin/sh
# Fetch DEX canister IDs from the public SNS registry and export env vars.
set -e
JSON=$(curl -fsSL https://sns-api.internetcomputer.org/api/v1/snses)
get_id() {
  echo "$JSON" | jq -r --arg name "$1" '.data[] | select(.name|test($name; "i")) | .root_canister_id' | head -n 1
}
export ICPSWAP_FACTORY=$(get_id ICPSwap)
export SONIC_ROUTER=$(get_id Sonic)
export INFINITY_VAULT=$(get_id InfinitySwap)
export SNS_DISTRIBUTOR=$(get_id Distributor)

echo "ICPSWAP_FACTORY=$ICPSWAP_FACTORY"
echo "SONIC_ROUTER=$SONIC_ROUTER"
echo "INFINITY_VAULT=$INFINITY_VAULT"
echo "SNS_DISTRIBUTOR=$SNS_DISTRIBUTOR"

cat > .env.generated <<EOF
ICPSWAP_FACTORY=$ICPSWAP_FACTORY
SONIC_ROUTER=$SONIC_ROUTER
INFINITY_VAULT=$INFINITY_VAULT
SNS_DISTRIBUTOR=$SNS_DISTRIBUTOR
EOF
