#!/usr/bin/env fish

for dir in . examples/*
    echo $dir
    cd "$dir"
    cargo clippy
end
