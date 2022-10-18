#!/bin/bash

prover=$(cat ../prover/neardev/dev-account)

near dev-deploy \
  --initFunction new --initArgs  "{\"prover_account\":\"$prover\"}" \
  --wasmFile target/wasm32-unknown-unknown/release/nft_connector.wasm
