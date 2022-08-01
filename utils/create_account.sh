#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Illegal number of parameters (shard_id, account_id)"
    exit 1
fi
echo "Assuming possession of treasury key..."

destination_master_account="$1.calimero"
prover="p.$destination_master_account"
near create-account "$2" \
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1/neard-rpc" \
  --networkId "$1" \
  --masterAccount $destination_master_account
