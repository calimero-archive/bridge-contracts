#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi
destination_master_account="$1.calimero.testnet"
contract_id="nft_connector.$destination_master_account"
locker=$(cat ../nft_connector/neardev/dev-account)

near call "$contract_id" set_locker \
  --accountId "$contract_id" \
  --args  "{\"locker_account\":\"$locker\"}" \
  --nodeUrl "https://api.development.calimero.network/api/v1/shards/$1-calimero-testnet/neard-rpc" \
  --networkId "$1"
