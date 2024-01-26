#!/opt/homebrew/bin/bash

set -euo pipefail

# until https://github.com/killercup/cargo-edit/pull/870 is merged
CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git cargo fetch

mapfile -t all < <(fd Cargo.toml)
roots=()
for file in "${all[@]}"; do
  if [[ $file == *"templates"* ]]; then
    continue
  fi
  roots+=("$(
    cd "$(dirname "$file")"
    cargo metadata --format-version 1 | jq -r '.workspace_root'
  )")
done

readarray -t sorted < <(printf '%s\n' "${roots[@]}" | sort -u)

for dir in "${sorted[@]}"; do
  (
    echo "---  Updating dependencies in $dir"
    cd "$dir"
    # until https://github.com/killercup/cargo-edit/pull/870 is merged
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=git cargo upgrade -i --verbose
    cargo update
  )
done
