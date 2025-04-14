#!/usr/bin/env bash

set -eux

cargo run --package shared --bin crux_cli -- \
    codegen --crate-name shared \
        --out-dir ./shared/generated \
        --java-package com.crux.example.counter.shared

cargo run --package shared --bin crux_cli -- \
    bindgen --crate-name shared \
        --out-dir ./shared/generated
