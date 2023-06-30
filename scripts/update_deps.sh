#!/bin/bash

cargo upgrade -i --verbose
cargo update

for example in ./examples/*; do
  (
    cd "$example" &&
      cargo upgrade -i --verbose &&
      cargo update
  )
done
