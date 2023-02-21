#!/bin/bash

RUSTFLAGS='--cfg reset -C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/light_client.wasm ../wasm/light_client_reset.wasm
