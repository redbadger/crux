#!/usr/bin/env fish

cargo build

for d in ../examples/hello_world
    echo ""
    echo "---------------"
    echo "Public API for $d"
    pushd $d
    ../../target/debug/crux codegen --lib shared
    popd
end
