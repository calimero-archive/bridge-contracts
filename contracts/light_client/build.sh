#!/bin/bash

RUSTFLAGS='--cfg reset' cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/light_client.wasm ../wasm/