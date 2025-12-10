#!/usr/bin/env fish

for dir in . crux_* examples/* examples/*/tauri/src-tauri
    echo "---  Updating dependencies in $dir"
    pushd "$dir"
    cargo update
    cargo upgrade --incompatible allow
    cargo update
    popd
end
