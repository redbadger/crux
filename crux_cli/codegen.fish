#!/usr/bin/env fish

cargo build
pushd ../examples/counter
../../target/debug/crux codegen --lib shared
popd
