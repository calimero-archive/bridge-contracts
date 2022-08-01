#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi
destination_master_account="$1.calimero"
contract_id="connector.$destination_master_account"
locker=$(cat ../ft_connector_source/neardev/dev-account)

near call "$contract_id" set_locker \
  --accountId "$contract_id" \
  --args  "{\"locker_account\":\"$locker\"}" \
  --nodeUrl "https://api-staging.calimero.network/api/v1/shards/$1/neard-rpc" \
  --networkId "$1"
