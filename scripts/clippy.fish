#!/usr/bin/env fish

for dir in . examples/*
    echo $dir
    cd "$dir"
    cargo clippy -- --no-deps -Dclippy::pedantic -Dwarnings; or return 1
end
