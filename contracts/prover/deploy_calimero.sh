#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters"
    exit 1
fi
destination_master_account="$1.calimero"
prover="p.$destination_master_account"

light_client="lc.$destination_master_account"
near deploy \
  --accountId "p.$destination_master_account" \
  --wasmFile target/wasm32-unknown-unknown/release/prover.wasm \
  --initFunction new --initArgs "{\"light_client_account_id\":\"$light_client\"}" \
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1-calimero/neard-rpc" \
  --networkId "$1"
