#!/usr/bin/env fish

set -l name (basename (status -f))

argparse --name $name h/help 'p/path=' -- $argv
or return 1

if test "$_flag_help"
    echo "Usage: $name --path <path-to-manifest-dir>"
    return 0
end

set -x RUSTC_BOOTSTRAP 1
set -x RUSTDOCFLAGS "-Z unstable-options --output-format=json --cap-lints=allow"

rm -rf target/doc # workspace
rm -rf "$_flag_path"/target/doc # not workspace

cargo doc --no-deps --lib --manifest-path "$_flag_path"/Cargo.toml
