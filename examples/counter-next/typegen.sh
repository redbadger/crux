#!/usr/bin/env bash

set -eux

pushd shared
cargo swift package --name Shared --platforms ios
pushd generated
rm -rf headers sources *.swift *.h *.modulemap
popd
popd

# cargo run --package shared --bin crux_cli --features cli -- \
#     codegen \
#         --java com.crux.example.counter.shared \
#         --swift SharedTypes \
#         --typescript shared_types

cargo run --package shared --bin crux_cli --features cli -- \
    bindgen --out-dir=./shared_types/generated
