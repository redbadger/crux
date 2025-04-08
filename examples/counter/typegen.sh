#!/usr/bin/env bash

cargo run --package shared --bin crux_cli -- \
    gen --lib shared \
        --output ./shared_types/generated \
        --java-package com.crux.example.counter
