#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi

prover=$(cat ../prover/neardev/dev-account)
destination_master_account="$1.calimero.testnet"
near dev-deploy \
  --initFunction new --initArgs  "{\"prover_account\":\"$prover\", \"source_master_account\": \"testnet\", \"destination_master_account\": \"$destination_master_account\"}" \
  --wasmFile target/wasm32-unknown-unknown/release/nft_connector_source.wasm
