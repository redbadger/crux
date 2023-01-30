#!/usr/bin/env bash

if [ "$#" -ne 1 ]; then
  echo "Usage (note: only call inside XCode!):"
  echo "$0 <FFI_TARGET>"
  exit 1
fi

# what to pass to cargo build -p, e.g. your_lib_ffi
FFI_TARGET=$1

set -euvx

RELFLAG=
if [[ "$CONFIGURATION" != "Debug" ]]; then
  RELFLAG=--release
fi

IS_SIMULATOR=0
if [ "${LLVM_TARGET_TRIPLE_SUFFIX-}" = "-simulator" ]; then
  IS_SIMULATOR=1
fi

export PATH="$PATH:$HOME/.cargo/bin"
export LIBRARY_PATH

for arch in $ARCHS; do
  case "$arch" in
  x86_64)
    if [ $IS_SIMULATOR -eq 0 ]; then
      echo "Building for x86_64, but not a simulator build. What's going on?" >&2
      exit 2
    fi

    # Intel iOS simulator
    export CFLAGS_x86_64_apple_ios="-target x86_64-apple-ios"
    LIBRARY_PATH="${LIBRARY_PATH-}:$(xcrun --sdk iphonesimulator --show-sdk-path)/usr/lib"
    cargo build -p "$FFI_TARGET" --lib $RELFLAG --target x86_64-apple-ios
    ;;

  arm64)
    if [ $IS_SIMULATOR -eq 0 ]; then
      # Hardware iOS targets
      LIBRARY_PATH="${LIBRARY_PATH-}:$(xcrun --sdk iphoneos --show-sdk-path)/usr/lib"
      cargo build -p "$FFI_TARGET" --lib $RELFLAG --target aarch64-apple-ios
    else
      LIBRARY_PATH="${LIBRARY_PATH-}:$(xcrun --sdk iphonesimulator --show-sdk-path)/usr/lib"
      cargo build -p "$FFI_TARGET" --lib $RELFLAG --target aarch64-apple-ios-sim
    fi
    ;;
  esac
done
