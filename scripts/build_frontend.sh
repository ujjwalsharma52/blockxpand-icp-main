#!/bin/sh
set -e
CID=${CANISTER_ID:-$(dfx canister id aggregator 2>/dev/null || true)}
if [ -z "$CID" ]; then
  echo "Error: CANISTER_ID not provided and dfx query failed" >&2
  exit 1
fi
mkdir -p frontend/dist
sed "s/<CANISTER_ID>/$CID/g" frontend/index.html > frontend/dist/index.html
echo "Generated frontend/dist/index.html with canister ID $CID"
