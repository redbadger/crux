#!/usr/bin/env fish

for dir in . examples/*
    echo "---  Updating dependencies in $dir"
    pushd "$dir"
    cargo update
    cargo +nightly update -Z unstable-options --breaking
    popd
end
