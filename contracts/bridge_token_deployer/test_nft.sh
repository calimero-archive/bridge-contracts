#!/bin/bash

BRIDGE_TOKEN=../nft_bridge_token.wasm RUSTFLAGS='-C link-arg=-s' cargo test -- --nocapture 2> /dev/null
