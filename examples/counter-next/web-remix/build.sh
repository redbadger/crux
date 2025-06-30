#!/usr/bin/env bash

export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'

pushd ../shared && RUST_LOG=info wasm-pack build --target web
