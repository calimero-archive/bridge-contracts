#!/bin/bash

prover=$(cat ../prover/neardev/dev-account)
near dev-deploy --wasmFile target/wasm32-unknown-unknown/release/ft_connector_destination.wasm --initFunction new --initArgs  "{\"prover_account\":\"$prover\"}"
