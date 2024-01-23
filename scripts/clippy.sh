#!/bin/bash

set -e

for dir in . ./examples/*; do
  (
    cd "$dir"
    cargo clippy
  )
done
