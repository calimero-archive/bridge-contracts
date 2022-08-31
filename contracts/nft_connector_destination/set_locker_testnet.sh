#!/bin/bash

locker=$(cat ../nft_connector_source/neardev/dev-account)
contract_id=$(cat ./neardev/dev-account)

near call "$contract_id" set_locker \
  --accountId "$contract_id" \
  --args  "{\"locker_account\":\"$locker\"}" \