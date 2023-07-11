#!/bin/bash

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp ../target/wasm32-unknown-unknown/release/bridge_token.wasm ../bridge_token_deployer/ft_bridge_token.wasm
