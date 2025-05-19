#!/usr/bin/env fish

for dir in . examples/*
    echo $dir
    pushd "$dir"
    cargo fmt --all --check
    cargo build --all-features; or return 1
    cargo nextest run --all-features; or return 1
    popd
end
