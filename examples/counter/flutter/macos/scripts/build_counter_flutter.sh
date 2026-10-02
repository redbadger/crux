#!/usr/bin/env bash
# Build counter_flutter (flutter_rust_bridge / Crux core) and embed it as a
# framework so Dart can dlopen(counter_flutter.framework/counter_flutter).
set -euo pipefail

export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}"

ROOT="$(cd "${SRCROOT}/.." && pwd)"
MANIFEST="${ROOT}/rust/Cargo.toml"
export CARGO_TARGET_DIR="${ROOT}/rust/target"

case "${CONFIGURATION:-Debug}" in
    Release) cargo_args=(--release) profile_dir=release ;;
    *) cargo_args=() profile_dir=debug ;;
esac

if [[ "${ARCHS:-}" == *"arm64"* ]]; then
    target=aarch64-apple-darwin
elif [[ "${ARCHS:-}" == *"x86_64"* ]]; then
    target=x86_64-apple-darwin
else
    target="$(rustc -vV | sed -n 's/^host: //p')"
fi

if ! rustup target list --installed | grep -q "^${target}\$"; then
    echo "error: Rust target '${target}' not installed. Run: rustup target add ${target}" >&2
    exit 1
fi

echo "Building counter_flutter (${target}, ${profile_dir})…"
cargo build --manifest-path "${MANIFEST}" "${cargo_args[@]}" --target "${target}"

dylib="${CARGO_TARGET_DIR}/${target}/${profile_dir}/libcounter_flutter.dylib"
if [[ ! -f "${dylib}" ]]; then
    dylib="${CARGO_TARGET_DIR}/${target}/${profile_dir}/deps/libcounter_flutter.dylib"
fi
if [[ ! -f "${dylib}" ]]; then
    echo "error: missing libcounter_flutter.dylib under ${CARGO_TARGET_DIR}/${target}/${profile_dir}/" >&2
    exit 1
fi

dest_root="${BUILT_PRODUCTS_DIR}/${FRAMEWORKS_FOLDER_PATH}"
fw="${dest_root}/counter_flutter.framework"
rm -rf "${fw}"
mkdir -p "${fw}/Versions/A/Resources"
cp "${dylib}" "${fw}/Versions/A/counter_flutter"
chmod +x "${fw}/Versions/A/counter_flutter"
ln -sf A "${fw}/Versions/Current"
ln -sf "Versions/Current/counter_flutter" "${fw}/counter_flutter"
ln -sf "Versions/Current/Resources" "${fw}/Resources"
cat >"${fw}/Versions/A/Resources/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleExecutable</key>
	<string>counter_flutter</string>
	<key>CFBundleIdentifier</key>
	<string>dev.crux.examples.counter.counterFlutter</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>counter_flutter</string>
	<key>CFBundlePackageType</key>
	<string>FMWK</string>
	<key>CFBundleShortVersionString</key>
	<string>1.0</string>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>MinimumOSVersion</key>
	<string>10.14</string>
</dict>
</plist>
PLIST

install_name_tool -id \
    "@executable_path/../Frameworks/counter_flutter.framework/Versions/A/counter_flutter" \
    "${fw}/Versions/A/counter_flutter"

echo "Embedded ${fw}"
