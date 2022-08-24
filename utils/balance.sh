#!/bin/bash

if [ "$#" -ne 3 ]; then
    echo "Illegal number of parameters (shard_id, token_account_id, account_id)"
    exit 1
fi
destination_master_account="$1.calimero.testnet"

near view "$2" ft_balance_of \
  --args  "{\"account_id\":\"$3\"}" \
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1-calimero-testnet/neard-rpc" \
  --networkId "$1"
