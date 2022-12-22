#!/bin/bash

BRIDGE_TOKEN=../ft_bridge_token.wasm RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/bridge_token_deployer.wasm ../wasm/ft_bridge_token_deployer.wasm
