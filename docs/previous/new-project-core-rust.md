# Rust Core

[Table of Contents](./new-project.md)

1. Make sure you have the following rust targets installed (e.g. `rustup target add <target-name>`)

   ```txt
   aarch64-apple-darwin
   aarch64-apple-ios
   aarch64-apple-ios-sim
   aarch64-linux-android
   wasm32-unknown-unknown
   x86_64-apple-ios
   ```

1. Install the `uniffi-bindgen` binary ...

   ```sh
   cargo install uniffi_bindgen
   ```

1. Create a new rust library ...

   ```sh
   cargo new --lib shared
   ```

1. Edit [`./Cargo.toml`](../Cargo.toml) to add the new library to the Cargo workspace

   ```toml
   [workspace]
   members = ["shared"]
   ```

1. Edit [`./shared/Cargo.toml`](../shared/Cargo.toml)

   Note that the crate type:

   1. `"lib"` is the default rust library when linking into a rust binary, e.g. for WebAssembly in the web variant
   1. `"staticlib"` is a static library (`libshared.a`) for including in the Swift iOS app variant
   1. `"cdylib"` is a c-abi dynamic library (`libshared.so`) for use with JNA when included in the Kotlin Android app variant

   ```toml
   [lib]
   crate-type = ["lib", "staticlib", "cdylib"]
   name = "shared"

   [dependencies]
   uniffi = "0.21.0"
   uniffi_macros = "0.21.0"

   [build-dependencies]
   uniffi_build = { version = "0.21.0", features = ["builtin-bindgen"] }
   ```

1. Create [`./shared/src/shared.udl`](../shared/src/shared.udl) ...

   ```txt
   namespace shared {
     u32 add(u32 left, u32 right);
   };
   ```

1. Create [`./shared/uniffi.toml`](../shared/uniffi.toml) ...

   ```toml
   [bindings.kotlin]
   package_name = "com.redbadger.crux_core.shared"
   cdylib_name = "shared"

   [bindings.swift]
   cdylib_name = "shared_ffi"
   omit_argument_labels = true
   ```

1. Create [`./shared/build.rs`](../shared/build.rs) ...

   ```rust,noplayground
   fn main() {
       uniffi_build::generate_scaffolding("./src/shared.udl").unwrap();
   }
   ```

1. Include the scaffolding in [`./shared/src/lib.rs`](../shared/src/lib.rs), and change types from `usize` to `u32` ...

   ```rust,noplayground
   uniffi_macros::include_scaffolding!("shared");

   pub fn add(left: u32, right: u32) -> u32 {
       left + right
   }
   ```

1. Make sure everything builds OK

   ```sh
   cargo build
   ```
