#!/opt/homebrew/bin/bash

set -euo pipefail

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
    cd "$dir"
    cargo fmt --all --check
    cargo build --all-features
    cargo nextest run --all-features
  )
done
