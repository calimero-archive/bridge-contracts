#!/bin/bash

near dev-deploy --wasmFile ../target/wasm32-unknown-unknown/release/light_client.wasm --initFunction new --initArgs '{"lock_duration":10,"replace_duration":2000000}'
