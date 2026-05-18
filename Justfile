# Crates published in order (dependencies before dependents)
publish_packages := "crux_cli crux_macros crux_core crux_http crux_kv crux_platform crux_time"

default: ci

# Build the root workspace
build:
    @echo '{{ style("command") }}build:{{ NORMAL }}'
    cargo build --all-features

# Check formatting, types, and linting in the root workspace
check:
    @echo '{{ style("command") }}check:{{ NORMAL }}'
    cargo fmt --all --check
    cargo check --all-features
    cargo clippy --all-targets -- --no-deps -Dclippy::pedantic -Dwarnings

# Clean build artefacts in the root workspace and all examples
clean:
    @echo '{{ style("command") }}clean:{{ NORMAL }}'
    cargo clean
    just examples/clean

# Fix formatting in the root workspace and all examples
fix:
    @echo '{{ style("command") }}fix:{{ NORMAL }}'
    cargo fmt --all
    just examples/fix

# Run tests locally (with cargo-insta snapshot review)
test:
    @echo '{{ style("command") }}test:{{ NORMAL }}'
    cargo insta test --review --test-runner nextest --all-features --lib

# Run CI workflow — check, build, and test the root workspace, then all examples
ci: check build
    @echo '{{ style("command") }}test:{{ NORMAL }}'
    cargo nextest run --all-features
    cargo test --doc --all-features
    just examples/ci

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
