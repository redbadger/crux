#!/usr/bin/env fish

for dir in crux_macros crux_core crux_http crux_kv crux_platform crux_time
    echo $dir
    cargo publish --package $dir
end
