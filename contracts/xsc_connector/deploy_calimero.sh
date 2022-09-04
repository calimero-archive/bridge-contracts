#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi

prover="prover.$1.calimero.testnet"
near deploy \
  --accountId "xsc-connector.$destination_master_account" \
  --initFunction new --initArgs  "{\"prover_account\":\"$prover\"}" \
  --wasmFile target/wasm32-unknown-unknown/release/xsc_connector.wasm
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1-calimero-testnet/neard-rpc" \
  --networkId "$1-calimero-testnet"