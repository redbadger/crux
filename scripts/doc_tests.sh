#!/opt/homebrew/bin/bash

set -euo pipefail

cargo test --doc --all-features
