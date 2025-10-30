#!/usr/bin/env bash

set -eux

# package our shared library for iOS, including bindgen for Swift
mkdir -p shared/generated/swift
pushd shared/generated/swift
cargo swift package --name Shared --platforms ios --features uniffi
rm -rf generated
popd

# uniffi proc macros need `libshared` for introspection
# (to support bindgen for Kotlin)
cargo build --package shared --features uniffi

# run typegen for Swift/Kotlin/TypeScript and bindgen for Kotlin
RUST_LOG=info cargo run --package shared --bin codegen --features codegen,facet_typegen
