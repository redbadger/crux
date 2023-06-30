#!/bin/bash

cargo clippy --fix --allow-staged

for example in ./examples/*; do
  (cd "$example" && cargo clippy --fix --allow-staged)
done
