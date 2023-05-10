#!/bin/bash

cargo nextest run --all-features

cargo test --doc --all-features

for example in ./examples/*; do
  (cd "$example" && cargo nextest run --all-features)
done
