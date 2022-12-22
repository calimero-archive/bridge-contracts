#!/bin/bash

BRIDGE_TOKEN=../bridge_token.wasm RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/ft_connector.wasm ../wasm/
