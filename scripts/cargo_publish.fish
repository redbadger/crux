#!/usr/bin/env fish

for e in crux_macros crux_http crux_kv crux_platform crux_time crux_core
    echo $e
    cargo publish --package $e
end
