#!/bin/bash

set -e

# until https://github.com/killercup/cargo-edit/pull/870 is merged
CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git cargo fetch

for dir in . ./examples/*; do
  (
    cd "$dir"
    # until https://github.com/killercup/cargo-edit/pull/870 is merged
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git cargo upgrade -i --verbose
    cargo update
  )
done
