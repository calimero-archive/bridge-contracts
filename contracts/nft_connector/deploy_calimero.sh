#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi
destination_master_account="$1.calimero.testnet"
prover="prover.$destination_master_account"
near deploy \
  --accountId "ft_connector.$destination_master_account" \
  --wasmFile target/wasm32-unknown-unknown/release/nft_connector.wasm \
  --initFunction new --initArgs "{\"prover_account\":\"$prover\"}" \
  --nodeUrl "https://api.development.calimero.network/api/v1/shards/$1-calimero-testnet/neard-rpc" \
  --networkId "$1"
