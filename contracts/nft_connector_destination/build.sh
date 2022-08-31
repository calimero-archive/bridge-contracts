#!/bin/bash

BRIDGE_TOKEN=../bridge_token.wasm cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/nft_connector_destination.wasm ../wasm/
