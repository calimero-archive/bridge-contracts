#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Illegal number of parameters (shard_id, account_id)"
    exit 1
fi
echo "Assuming possession of treasury key..."

destination_master_account="$1.calimero"
near create-account "$2" \
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1-calimero/neard-rpc" \
  --networkId "$1" \
  --masterAccount $destination_master_account
