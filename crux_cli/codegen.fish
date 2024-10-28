#!/usr/bin/env fish

cargo build

for d in ../examples/hello_world
    pushd $d
    ../../target/debug/crux codegen --lib shared
    popd
end
