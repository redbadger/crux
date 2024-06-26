#!/usr/bin/env fish

# until https://github.com/killercup/cargo-edit/pull/870 is merged
CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git cargo fetch

for dir in . examples/*
    echo "---  Updating dependencies in $dir"
    pushd "$dir"
    # until https://github.com/killercup/cargo-edit/pull/870 is merged
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git cargo upgrade -i --verbose
    cargo update
    popd
end
