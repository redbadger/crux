#!/usr/bin/env bash

cargo run --package shared --bin crux_cli -- \
    gen --crate-name shared \
        --out-dir ./shared/generated \
        --java-package com.crux.example.counter

cargo run --package shared --bin crux_cli -- \
    ffi --crate-name shared \
        --out-dir ./shared/generated
