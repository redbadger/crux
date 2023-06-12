#!/bin/bash

# run the unit tests
cargo nextest run --all-features

# run the integration tests
cargo test --test '*'

# run the doc tests
cargo test --doc --all-features

for example in ./examples/*; do
  (cd "$example" && cargo nextest run --all-features)
done
