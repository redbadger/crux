#!/bin/bash

set -e

for dir in . ./examples/*; do
  (
    cd "$dir"
    cargo clean
    rm -rf shared_types/generated
    rm -rf iOS/generated
  )
done
