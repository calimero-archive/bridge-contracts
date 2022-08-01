#!/bin/bash
if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters"
    exit 1
fi
destination_master_account="$1.calimero"

near deploy \
  --accountId "lc.$destination_master_account" \
  --wasmFile target/wasm32-unknown-unknown/release/light_client.wasm \
  --initFunction new --initArgs '{"lock_duration":10,"replace_duration":2000000}' \
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1/neard-rpc" \
  --networkId "$1"