#!/bin/bash

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters (shard_id)"
    exit 1
fi
destination_master_account="$1.calimero.testnet"
locker="ft_connector.$destination_master_account"
contract_id=$(cat ../ft_connector/neardev/dev-account)

near call "$contract_id" set_locker \
  --accountId "$contract_id" \
  --args  "{\"locker_account\":\"$locker\"}" \
