#!/bin/bash

set -euxo pipefail

for dir in ./examples/*; do
  (
    cd "$dir"
    cargo fmt --all --check
    cargo build --all-features
    cargo nextest run --all-features
  )
done
