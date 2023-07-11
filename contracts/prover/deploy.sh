#!/bin/bash
rm -rf neardev
light_client=$(cat ../light_client/neardev/dev-account)
near dev-deploy --wasmFile ../target/wasm32-unknown-unknown/release/prover.wasm --initFunction new --initArgs "{\"light_client_account_id\":\"$light_client\"}"
