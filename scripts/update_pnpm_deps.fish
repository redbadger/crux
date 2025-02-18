#!/usr/bin/env fish

for e in examples/*/web-* examples/*/tauri
    if test -e $e/package.json
        echo $e
        pushd $e && pnpm update --latest && popd
    end
end
