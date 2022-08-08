#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi
destination_master_account="$1.calimero"
prover="p.$destination_master_account"
near deploy \
  --accountId "connector.$destination_master_account" \
  --wasmFile target/wasm32-unknown-unknown/release/ft_connector_destination.wasm \
  --initFunction new --initArgs "{\"prover_account\":\"$prover\", \"source_master_account\": \"testnet\", \"destination_master_account\": \"$destination_master_account\"}" \
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1/neard-rpc" \
  --networkId "$1"
