#!/usr/bin/env fish

for e in . examples/*
    echo $e
    pushd $e
    cargo update
    popd
end
