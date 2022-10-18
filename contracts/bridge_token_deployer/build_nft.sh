#!/bin/bash
rm target/wasm32-unknown-unknown/release/bridge_token_deployer.wasm
BRIDGE_TOKEN=../nft_bridge_token.wasm cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/bridge_token_deployer.wasm ../wasm/nft_bridge_token_deployer.wasm
