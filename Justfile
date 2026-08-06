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
check: check-versions
    @echo '{{ style("command") }}check:{{ NORMAL }}'
    cargo fmt --all --check
    cargo check --all-features
    cargo clippy --all-targets -- --no-deps -Dclippy::pedantic -Dclippy::nursery -Dwarnings

# Verify every BoltFFI, Binaryen and facet version pin agrees with lib.just.
#
# The versions are spread across workspace manifests, package.json files, CI,
# the book and renovate.json, and they all have to move together — a partial
# bump leaves the toolchain silently mismatched rather than failing loudly.
# lib.just is the single source of truth; this recipe holds everything to it.
#
# facet is the worst of the three to get wrong. It is pinned in ten places (the
# workspace manifest, all six example shared crates, the book, and two lines of
# renovate.json), and a partial bump does not surface as a version conflict —
# cargo happily resolves two facet_core versions, and you get
# `RenderOperation: Facet<'_> is not satisfied` from whichever crate ended up on
# the wrong side. The facet pin is also locked to facet_generate, which itself
# depends on `facet =<facet_version>`, so the two can only move together.
[script('bash')]
check-versions:
    set -euo pipefail
    echo '{{ style("command") }}check-versions:{{ NORMAL }}'

    boltffi=$(sed -nE 's/^boltffi_version := "(.*)"/\1/p' lib.just)
    binaryen=$(sed -nE 's/^binaryen_version := "(.*)"/\1/p' lib.just)
    facet=$(sed -nE 's/^facet_version := "(.*)"/\1/p' lib.just)
    if [ -z "$boltffi" ] || [ -z "$binaryen" ] || [ -z "$facet" ]; then
        echo "  {{ style("error") }}✗ could not read the expected versions from lib.just{{ NORMAL }}"
        exit 1
    fi

    status=0
    checked=0
    while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        file=${hit%%:*}
        found=$(printf '%s' "${hit#*:}" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n 1)
        checked=$((checked + 1))
        if [ "$found" != "$boltffi" ]; then
            echo "  {{ style("error") }}✗ $file pins BoltFFI $found, expected $boltffi{{ NORMAL }}"
            status=1
        fi
    done <<PINS
    $(grep -HoE '^boltffi = "=[0-9]+\.[0-9]+\.[0-9]+"' Cargo.toml examples/*/Cargo.toml || true)
    $(grep -HoE '"@boltffi/runtime": "[0-9]+\.[0-9]+\.[0-9]+"' examples/*/*/package.json || true)
    $(grep -rHoE "boltffi_cli --version '=[0-9]+\.[0-9]+\.[0-9]+'" .github/workflows docs/src lib.just || true)
    $(grep -rHoE '^boltffi = "=[0-9]+\.[0-9]+\.[0-9]+"' docs/src || true)
    $(grep -HoE '^boltffi_version := "[0-9]+\.[0-9]+\.[0-9]+"' examples/counter/windows/Justfile || true)
    $(grep -A2 -H '"boltffi", "boltffi_cli"' renovate.json | grep -E 'allowedVersions' || true)
    PINS

    facet_checked=0
    while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        file=${hit%%:*}
        found=$(printf '%s' "${hit#*:}" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n 1)
        facet_checked=$((facet_checked + 1))
        if [ "$found" != "$facet" ]; then
            echo "  {{ style("error") }}✗ $file pins facet $found, expected $facet{{ NORMAL }}"
            status=1
        fi
    done <<FACET_PINS
    $(grep -HoE '^facet = "=[0-9]+\.[0-9]+\.[0-9]+"' Cargo.toml examples/*/shared/Cargo.toml || true)
    $(grep -HoE '^facet = \{ version = "=[0-9]+\.[0-9]+\.[0-9]+"' examples/*/shared/Cargo.toml || true)
    $(grep -rHoE '^facet = "=[0-9]+\.[0-9]+\.[0-9]+"' docs/src || true)
    $(grep -HoE 'Pin facet to [0-9]+\.[0-9]+\.[0-9]+' renovate.json || true)
    $(grep -F -A3 -H '"matchPackageNames": ["facet"],' renovate.json | grep -E 'allowedVersions' || true)
    FACET_PINS

    if [ "$facet_checked" -lt 10 ]; then
        echo "  {{ style("error") }}✗ only found $facet_checked facet pins, expected at least 10 — has a pin site moved or been renamed?{{ NORMAL }}"
        status=1
    fi

    ci_binaryen=$(sed -nE 's/^ *binaryen_version=([0-9]+).*/\1/p' .github/workflows/examples.yaml | head -n 1)
    if [ "$ci_binaryen" != "$binaryen" ]; then
        echo "  {{ style("error") }}✗ examples.yaml installs Binaryen $ci_binaryen, expected $binaryen{{ NORMAL }}"
        status=1
    fi

    if [ "$status" -eq 0 ]; then
        echo "  ✓ $checked BoltFFI pins at $boltffi, Binaryen at $binaryen, $facet_checked facet pins at $facet"
    fi
    exit $status

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
