#!/usr/bin/env fish

cargo build

for d in ../examples/cat_facts
    pushd $d
    ../../target/debug/crux codegen --lib shared
    popd
end
