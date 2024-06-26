#!/usr/bin/env fish

for dir in . examples/*
    echo $dir
    pushd "$dir"
    cargo clean
    rm -rf shared_types/generated
    rm -rf iOS/generated
    popd
end
