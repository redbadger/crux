# Shared justfile helpers for Crux.
# Import with: mod lib '../lib.just' (adjust path as needed)

# Check if a tool is installed, report status (does not fail build)
[script('bash'), no-cd]
check-tool tool install_cmd:
    if command -v {{ tool }} >/dev/null 2>&1; then
        echo '  ✓ {{ tool }}'
    else
        echo '  {{ style("error") }}✗ {{ tool }}{{ NORMAL }} — install with: {{ install_cmd }}'
    fi

# Check if a tool is installed, report status, and fail if missing
[script('bash'), no-cd]
require-tool tool install_cmd:
    if command -v {{ tool }} >/dev/null 2>&1; then
        echo '  ✓ {{ tool }}'
    else
        echo '  {{ style("error") }}✗ {{ tool }}{{ NORMAL }} — install with: {{ install_cmd }}'
        exit 1
    fi

# Check if a file exists, report status (does not fail build)
[script('bash'), no-cd]
check-file file setup_cmd:
    if [ -f {{ file }} ]; then
        echo '  ✓ {{ file }}'
    else
        echo '  {{ style("error") }}✗ {{ file }}{{ NORMAL }} — create with: {{ setup_cmd }}'
    fi

# Check if a Rust target is installed
[script('bash'), no-cd]
check-rust-target target:
    if rustup target list --installed 2>/dev/null | grep -q '{{ target }}'; then
        echo '  ✓ {{ target }}'
    else
        echo '  {{ style("error") }}✗ {{ target }}{{ NORMAL }} — install with: rustup target add {{ target }}'
    fi

# (internal) check java version is between 17 and 24
[script('bash'), no-cd]
_check-java-version:
    if ! command -v java >/dev/null 2>&1; then exit 0; fi
    ver=$(java -version 2>&1 | head -1 | sed -E 's/.*"([0-9]+).*/\1/')
    if [ "$ver" -ge 17 ] && [ "$ver" -le 24 ] 2>/dev/null; then
        echo "  ✓ java version $ver (17–24)"
    else
        echo "  {{ style("error") }}✗ java version $ver — need 17–24 (Gradle does not support 25+){{ NORMAL }}"
    fi

# Check tools for Apple shells
[no-cd]
check-apple:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-tool boltffi "cargo install boltffi_cli --version '=0.25.0'"
    @just lib::check-tool xcodegen "brew install xcodegen"
    @just lib::check-tool xcodebuild "install Xcode from the App Store"
    @just lib::check-tool xcrun "xcode-select --install"
    @just lib::check-tool swiftlint "brew install swiftlint"
    @just lib::check-rust-target aarch64-apple-ios
    @just lib::check-rust-target aarch64-apple-ios-sim
    @just lib::check-rust-target x86_64-apple-ios

# Check tools for Windows / WinUI3 shells
[no-cd]
check-windows:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-tool boltffi "cargo install boltffi_cli --version '=0.25.0'"
    @just lib::check-tool dotnet "install the .NET 10 SDK: https://dotnet.microsoft.com/download"
    @just lib::_check-vswhere
    @just lib::check-rust-target x86_64-pc-windows-msvc

# (internal) check for Visual Studio via vswhere at its canonical install path
[script('bash'), no-cd]
_check-vswhere:
    vswhere='/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe'
    if [ -f "$vswhere" ]; then
        echo '  ✓ Visual Studio (vswhere)'
    else
        echo '  {{ style("error") }}✗ Visual Studio vswhere{{ NORMAL }} — install Visual Studio 2022+ with the Windows App SDK workload'
    fi

# Check tools for Android shells
[no-cd]
check-android:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-tool boltffi "cargo install boltffi_cli --version '=0.25.0'"
    @just lib::check-tool rustup "https://rustup.rs"
    @just lib::check-tool java "install JDK 17+"
    @just lib::_check-java-version
    @just lib::check-tool python3 "install Python 3"
    @just lib::check-tool sdkmanager "install Android SDK command-line tools"
    @just lib::check-rust-target aarch64-linux-android
    @just lib::check-rust-target x86_64-linux-android

# Check tools for web shells using BoltFFI WASM packages (nextjs, react-router)
[no-cd]
check-web-wasm:
    @just lib::check-tool cargo "https://rustup.rs"
    @just lib::check-tool boltffi "cargo install boltffi_cli --version '=0.25.0'"
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
[unix, no-cd]
open-android-studio:
    echo '{{ style("command") }}open:{{ NORMAL }}'
    open -b com.google.android.studio .

[windows, no-cd]
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
