#!/bin/bash

set -e

# run the tests
cargo nextest run --all-features

# run the doc tests
cargo test --doc --all-features

for example in ./examples/*; do
  (cd "$example" && cargo nextest run --all-features)
done
