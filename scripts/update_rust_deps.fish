#!/usr/bin/env fish

for dir in . examples/* examples/*/tauri/src-tauri
    echo "---  Updating dependencies in $dir"
    pushd "$dir"
    cargo update
    cargo upgrade -i
    cargo +nightly update -Z unstable-options --breaking
    cargo update
    popd
end
