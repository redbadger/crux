# Crates published in order (dependencies before dependents)
publish_packages := "crux_macros crux_core crux_http crux_kv crux_time"

# Recipes here cover the root workspace only, so they stay fast enough for a
# tight edit loop. The examples need every platform toolchain (Xcode, Android
# SDK, .NET, trunk, dioxus, GTK) and are opt-in via the `*-examples` recipes —
# in real CI they fan out across separate runners, one shell per job.

default: dev

# Local development loop — fix, check, build, and test the root workspace
dev: fix check build test

# Build the root workspace
build:
    @echo '{{ style("command") }}build:{{ NORMAL }}'
    cargo build --all-features

# Check formatting, types, and linting in the root workspace
check:
    @echo '{{ style("command") }}check:{{ NORMAL }}'
    cargo fmt --all --check
    cargo check --all-features
    cargo clippy --all-targets -- --no-deps -Dclippy::pedantic -Dclippy::nursery -Dwarnings

# Check formatting and lint in all examples (heavy — needs every platform toolchain)
check-examples:
    @echo '{{ style("command") }}check-examples:{{ NORMAL }}'
    just examples/check

# Clean build artefacts in the root workspace and all examples
clean:
    @echo '{{ style("command") }}clean:{{ NORMAL }}'
    cargo clean
    just examples/clean

# Fix formatting in the root workspace
fix:
    @echo '{{ style("command") }}fix:{{ NORMAL }}'
    cargo fmt --all

# Fix formatting in all examples (heavy — needs every platform toolchain)
fix-examples:
    @echo '{{ style("command") }}fix-examples:{{ NORMAL }}'
    just examples/fix

# Run tests locally (with cargo-insta snapshot review)
test:
    @echo '{{ style("command") }}test:{{ NORMAL }}'
    cargo insta test --review --test-runner nextest --all-features --lib

# Mirror the `build` CI workflow — root workspace only, no examples
ci: check build
    @echo '{{ style("command") }}test:{{ NORMAL }}'
    cargo nextest run --all-features
    cargo test --doc --all-features
    @echo '{{ style("command") }}test (crux_http with http-types compat feature):{{ NORMAL }}'
    cargo nextest run -p crux_http --features crux_http/http-types
    cargo test --doc -p crux_http --features crux_http/http-types
    @echo '{{ style("command") }}check (crux_http for wasm32-unknown-unknown):{{ NORMAL }}'
    cargo check -p crux_http --target wasm32-unknown-unknown

# Mirror the `examples` CI workflow — every example x shell, serially (very heavy)
ci-examples:
    @echo '{{ style("command") }}ci-examples:{{ NORMAL }}'
    # CI spreads this across many runners, each with only its platform's toolchain
    just examples/ci

# Everything CI runs — root workspace and all examples
ci-all: ci ci-examples

# Run doc tests
test-doc:
    @echo '{{ style("command") }}test-doc:{{ NORMAL }}'
    cargo test --doc --all-features

# Update Cargo lockfiles across all workspaces (safe — stays within existing constraints)
update:
    @echo '{{ style("command") }}update:{{ NORMAL }}'
    cargo update
    just examples/update

# Upgrade Cargo dependency constraints and update lockfiles across all workspaces
# Requires: cargo install cargo-edit
[script('bash')]
update-deps:
    set -euo pipefail
    echo '{{ style("command") }}update-deps:{{ NORMAL }}'
    for dir in . crux_*/; do
        [[ -f "$dir/Cargo.toml" ]] || continue
        echo "  ~ ${dir%/}"
        (cd "$dir" && cargo upgrade --incompatible allow)
    done
    cargo update
    just examples/update-deps

# Update pnpm dependencies to latest across all web and tauri shells
update-pnpm-deps:
    @echo '{{ style("command") }}update-pnpm-deps:{{ NORMAL }}'
    just examples/update-pnpm-deps

# Publish crates interactively — asks confirmation per crate, then publishes and tags
[script('bash')]
publish:
    set -euo pipefail
    packages="{{ publish_packages }}"
    for pkg in $packages; do
        version=$(cargo pkgid --package "$pkg" | sed 's/.*[#@]//')
        tag="${pkg}-v${version}"
        printf '\nPublish %s? [y/N] ' "$tag"
        read answer
        case "$answer" in
            [Yy]*)
                echo "Publishing $tag..."
                cargo publish --package "$pkg"
                git push origin :refs/tags/"$tag"
                git tag --force "$tag"
                git push origin tag "$tag"
                ;;
            *)
                echo "$pkg skipped"
                ;;
        esac
    done
