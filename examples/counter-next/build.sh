#!/usr/bin/env bash

set -eux

pushd shared
cargo swift package --name Shared --platforms ios
pushd generated
# remove stuff we don't need
rm -rf headers sources *.swift *.h *.modulemap
popd
popd

cargo build --package shared
RUST_LOG=info cargo run --package shared --bin codegen --features cli,facet_typegen
