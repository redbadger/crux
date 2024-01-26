#!/usr/bin/env fish

for e in crux_macros crux_core crux_http crux_kv crux_platform crux_time
    echo $e
    cargo publish --package $e
end
