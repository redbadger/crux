#!/bin/bash

set -euxo pipefail

for dir in . ./examples/*; do
  (
    cd "$dir"
    cargo nextest run --all-features
    cargo test --doc --all-features
    cargo fmt --all --check
  )
done
