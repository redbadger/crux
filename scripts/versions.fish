#!/usr/bin/env fish

for dir in crux_macros crux_core crux_http crux_kv crux_platform crux_time
    pushd $dir
    echo {$dir}-v(cargo pkgid | cut -d "#" -f2)
    popd
end
