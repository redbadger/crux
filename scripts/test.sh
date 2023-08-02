#!/bin/bash

set -euxo pipefail

cargo fmt --all --check
cargo nextest run --all-features
cargo test --doc --all-features

for dir in ./examples/*; do
  (
    cd "$dir"
    cargo fmt --all --check
    cargo nextest run --all-features
  )
done
