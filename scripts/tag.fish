#!/usr/bin/env fish

git checkout master

for dir in crux_macros crux_core crux_http crux_kv crux_platform crux_time
    pushd $dir
    git tag {$dir}-v(cargo pkgid | cut -d "#" -f2)
    popd
end
