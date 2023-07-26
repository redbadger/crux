#!/bin/bash

set -e

for dir in . ./examples/*; do
  (
    cd "$dir"
    cargo upgrade -i --verbose
    cargo update
  )
done
