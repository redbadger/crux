#!/usr/bin/env fish

cargo build

for d in ../examples/simple_counter
    pushd $d
    ../../target/debug/crux codegen --lib shared
    popd
end
