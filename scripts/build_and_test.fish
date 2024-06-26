#!/usr/bin/env fish

for dir in . examples/*
    echo $dir
    pushd "$dir"
    cargo fmt --all --check
    cargo build --all-features
    cargo nextest run --all-features
    popd
end
