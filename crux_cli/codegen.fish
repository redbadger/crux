#!/usr/bin/env fish

cargo build

for d in ../examples/*
    echo ""
    echo "---------------"
    echo "Public API for $d"
    pushd $d
    ../../target/debug/crux codegen --lib shared
    popd
end
