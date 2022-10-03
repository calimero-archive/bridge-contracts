#!/bin/bash

connector=$(cat ../ft_connector_destination/neardev/dev-account)
near dev-deploy \
  --initFunction new --initArgs  "{\"bridge_account\":\"$connector\", \"source_master_account\": \"testnet\"}" \
  --wasmFile target/wasm32-unknown-unknown/release/bridge_token_deployer.wasm
