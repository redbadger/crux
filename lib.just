# Shared justfile helpers for Crux.
# Import with: mod lib '../lib.just' (adjust path as needed)

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

boltffi_version := "0.29.3"

# Minimum Binaryen release providing --enable-bulk-memory-opt, which BoltFFI passes
binaryen_version := "123"

# Check if a tool is installed, report status (does not fail build)
[no-cd]
[unix]
[script('bash')]
check-tool tool install_cmd:
    if command -v {{ tool }} >/dev/null 2>&1; then
        echo '  ✓ {{ tool }}'
    else
        echo '  {{ style("error") }}✗ {{ tool }}{{ NORMAL }} — install with: {{ install_cmd }}'
    fi

[no-cd]
[windows]
check-tool tool install_cmd:
    @if (Get-Command "{{ tool }}" -ErrorAction SilentlyContinue) { \
        Write-Host '  ✓ {{ tool }}' \
    } else { \
        Write-Host '  {{ style("error") }}✗ {{ tool }}{{ NORMAL }} — install with: {{ install_cmd }}' \
    }

# Check if a tool is installed, report status, and fail if missing
[no-cd]
[unix]
[script('bash')]
require-tool tool install_cmd:
    if command -v {{ tool }} >/dev/null 2>&1; then
        echo '  ✓ {{ tool }}'
    else
        echo '  {{ style("error") }}✗ {{ tool }}{{ NORMAL }} — install with: {{ install_cmd }}'
        exit 1
    fi

[no-cd]
[windows]
require-tool tool install_cmd:
    @if (Get-Command "{{ tool }}" -ErrorAction SilentlyContinue) { \
        Write-Host '  ✓ {{ tool }}' \
    } else { \
        Write-Host '  {{ style("error") }}✗ {{ tool }}{{ NORMAL }} — install with: {{ install_cmd }}' ; \
        exit 1 \
    }

# Check that the installed BoltFFI CLI matches the version used by the examples
[no-cd]
[unix]
[script('bash')]
check-boltffi:
    install_cmd="cargo install boltffi_cli --version '={{ boltffi_version }}' --locked"
    if ! command -v boltffi >/dev/null 2>&1; then
        echo "  {{ style("error") }}✗ boltffi{{ NORMAL }} — install with: $install_cmd"
        exit 0
    fi
    actual=$(boltffi --version 2>/dev/null | head -n 1)
    expected="boltffi {{ boltffi_version }}"
    if [ "$actual" = "$expected" ]; then
        echo "  ✓ $actual"
    else
        if [ -z "$actual" ]; then actual='boltffi (unknown version)'; fi
        echo "  {{ style("error") }}✗ $actual{{ NORMAL }} — expected $expected; install with: $install_cmd"
    fi

[no-cd]
[windows]
check-boltffi:
    @$install_cmd = "cargo install boltffi_cli --version '={{ boltffi_version }}' --locked" ; \
    if (!(Get-Command boltffi -ErrorAction SilentlyContinue)) { \
        Write-Host "  {{ style("error") }}✗ boltffi{{ NORMAL }} — install with: $install_cmd" ; \
        exit 0 \
    } ; \
    $actual = & boltffi --version 2>$null | Select-Object -First 1 ; \
    $expected = "boltffi {{ boltffi_version }}" ; \
    if ($actual -eq $expected) { \
        Write-Host "  ✓ $actual" \
    } else { \
        if (-not $actual) { $actual = 'boltffi (unknown version)' } ; \
        Write-Host "  {{ style("error") }}✗ $actual{{ NORMAL }} — expected $expected; install with: $install_cmd" \
    }

# Check if a file exists, report status (does not fail build)
[no-cd]
[unix]
[script('bash')]
check-file file setup_cmd:
    if [ -f {{ file }} ]; then
        echo '  ✓ {{ file }}'
    else
        echo '  {{ style("error") }}✗ {{ file }}{{ NORMAL }} — create with: {{ setup_cmd }}'
    fi

[no-cd]
[windows]
check-file file setup_cmd:
    @if (Test-Path "{{ file }}" -PathType Leaf) { \
        Write-Host '  ✓ {{ file }}' \
    } else { \
        Write-Host '  {{ style("error") }}✗ {{ file }}{{ NORMAL }} — create with: {{ setup_cmd }}' \
    }

# Check if a Rust target is installed
[no-cd]
[unix]
[script('bash')]
check-rust-target target:
    if rustup target list --installed 2>/dev/null | grep -q '{{ target }}'; then
        echo '  ✓ {{ target }}'
    else
        echo '  {{ style("error") }}✗ {{ target }}{{ NORMAL }} — install with: rustup target add {{ target }}'
    fi

[no-cd]
[windows]
check-rust-target target:
    @if ((rustup target list --installed 2>$null) -like '*{{ target }}*') { \
        Write-Host '  ✓ {{ target }}' \
    } else { \
        Write-Host '  {{ style("error") }}✗ {{ target }}{{ NORMAL }} — install with: rustup target add {{ target }}' \
    }

# (internal) check java version is between 17 and 24
[no-cd]
[unix]
[script('bash')]
_check-java-version:
    if ! command -v java >/dev/null 2>&1; then exit 0; fi
    ver=$(java -version 2>&1 | head -1 | sed -E 's/.*"([0-9]+).*/\1/')
    if [ "$ver" -ge 17 ] && [ "$ver" -le 24 ] 2>/dev/null; then
        echo "  ✓ java version $ver (17–24)"
    else
        echo "  {{ style("error") }}✗ java version $ver — need 17–24 (Gradle does not support 25+){{ NORMAL }}"
    fi

[no-cd]
[windows]
_check-java-version:
    @if (!(Get-Command java -ErrorAction SilentlyContinue)) { exit 0 } ; \
    $ver_str = & java -version 2>&1 | Select-Object -First 1 ; \
    if ($ver_str -match '"([0-9]+)') { \
        $ver = [int]$Matches[1] ; \
        if ($ver -ge 17 -and $ver -le 24) { \
            Write-Host "  ✓ java version $ver (17–24)" \
        } else { \
            Write-Host "  {{ style("error") }}✗ java version $ver — need 17–24 (Gradle does not support 25+){{ NORMAL }}" \
        } \
    } else { \
        Write-Host "  {{ style("error") }}✗ Could not parse java version from: $ver_str{{ NORMAL }}" \
    }

# (internal) check Binaryen's wasm-opt is new enough for BoltFFI
[no-cd]
[unix]
[script('bash')]
_check-wasm-opt-version:
    if ! command -v wasm-opt >/dev/null 2>&1; then exit 0; fi
    ver=$(wasm-opt --version 2>/dev/null | head -n 1 | sed -E 's/.*version ([0-9]+).*/\1/')
    if [ "$ver" -ge {{ binaryen_version }} ] 2>/dev/null; then
        echo "  ✓ wasm-opt version $ver (>= {{ binaryen_version }})"
    else
        echo "  {{ style("error") }}✗ wasm-opt version $ver — need {{ binaryen_version }}+ (BoltFFI passes --enable-bulk-memory-opt, added in Binaryen {{ binaryen_version }}){{ NORMAL }}"
    fi

[no-cd]
[windows]
_check-wasm-opt-version:
    @if (!(Get-Command wasm-opt -ErrorAction SilentlyContinue)) { exit 0 } ; \
    $ver_str = & wasm-opt --version 2>$null | Select-Object -First 1 ; \
    if ($ver_str -match 'version ([0-9]+)') { \
        $ver = [int]$Matches[1] ; \
        if ($ver -ge {{ binaryen_version }}) { \
            Write-Host "  ✓ wasm-opt version $ver (>= {{ binaryen_version }})" \
        } else { \
            Write-Host "  {{ style("error") }}✗ wasm-opt version $ver — need {{ binaryen_version }}+ (BoltFFI passes --enable-bulk-memory-opt, added in Binaryen {{ binaryen_version }}){{ NORMAL }}" \
        } \
    } else { \
        Write-Host "  {{ style("error") }}✗ Could not parse wasm-opt version from: $ver_str{{ NORMAL }}" \
    }

# Check tools for Apple shells
[no-cd]
check-apple:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-boltffi
    @just lib::check-tool xcodegen "brew install xcodegen"
    @just lib::check-tool xcodebuild "install Xcode from the App Store"
    @just lib::check-tool xcrun "xcode-select --install"
    @just lib::check-tool swiftlint "brew install swiftlint"
    @just lib::check-rust-target aarch64-apple-ios
    @just lib::check-rust-target aarch64-apple-ios-sim
    @just lib::check-rust-target x86_64-apple-ios

# Check tools for Windows / WinUI3 shells
[no-cd]
[windows]
check-windows:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-boltffi
    @just lib::check-tool dotnet "install the .NET 10 SDK: https://dotnet.microsoft.com/download"
    @just lib::_check-vswhere
    @just lib::check-rust-target x86_64-pc-windows-msvc
    @just lib::check-rust-target aarch64-pc-windows-msvc

# (internal) check for Visual Studio via vswhere at its canonical install path.
# Windows-only: there is no [unix] variant because the check is meaningless off
# Windows, and [unix] excludes Windows (the attribute keys off just's target OS,
# not the interpreter, so a bash variant could never run where it matters).
[no-cd]
[windows]
_check-vswhere:
    @if (Test-Path "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe" -PathType Leaf) { \
        Write-Host '  ✓ Visual Studio (vswhere)' \
    } else { \
        Write-Host '  {{ style("error") }}✗ Visual Studio vswhere{{ NORMAL }} — install Visual Studio 2022+ with the Windows App SDK workload' \
    }

# Check tools for Android shells
[no-cd]
check-android:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-boltffi
    @just lib::check-tool rustup "https://rustup.rs"
    @just lib::check-tool java "install JDK 17+"
    @just lib::_check-java-version
    @just lib::check-tool python3 "install Python 3"
    @just lib::check-tool sdkmanager "install Android SDK command-line tools"
    @just lib::check-rust-target aarch64-linux-android
    @just lib::check-rust-target x86_64-linux-android
    @just lib::check-rust-target armv7-linux-androideabi
    @just lib::check-rust-target i686-linux-android

# Check tools for web shells using BoltFFI WASM packages (nextjs, react-router)
[no-cd]
check-web-wasm:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-boltffi
    @just lib::check-tool wasm-opt "install Binaryen: brew install binaryen"
    @just lib::_check-wasm-opt-version
    @just lib::check-tool node "https://nodejs.org"
    @just lib::check-tool pnpm "npm install -g pnpm"
    @just lib::check-rust-target wasm32-unknown-unknown

# Check tools for web shells using trunk (leptos, yew)
[no-cd]
check-web-trunk:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-tool trunk "cargo install trunk"
    @just lib::check-rust-target wasm32-unknown-unknown

# Open the current directory in Android Studio
[no-cd]
[unix]
open-android-studio:
    echo '{{ style("command") }}open:{{ NORMAL }}'
    open -b com.google.android.studio .

[no-cd]
[windows]
open-android-studio:
    @echo '{{ style("command") }}open:{{ NORMAL }}'
    studio64 .

# Check tools for shared crate
[no-cd]
check-shared:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-tool cargo-nextest "cargo install cargo-nextest"

# Include BoltFFI's Node entrypoint files when pnpm packs the generated local package.
[no-cd]
fix-boltffi-wasm-package package_json:
    node -e "const fs = require('node:fs'); const path = process.argv[1]; const pkg = JSON.parse(fs.readFileSync(path, 'utf8')); const files = new Set(pkg.files ?? []); for (const file of ['shared_node.js', 'shared_node.d.ts', 'shared_node.js.map']) files.add(file); pkg.files = [...files]; fs.writeFileSync(path, JSON.stringify(pkg, null, 2) + '\n');" {{ package_json }}
