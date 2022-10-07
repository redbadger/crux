# Rust Multi-platform Mobile (RMM)

## Shared rust library

1. create new library

   ```sh
   cargo new --lib shared
   ```

1. add to `Cargo.toml`

   ```toml
    [lib]
    crate-type = ["lib", "cdylib"]
    name = "shared"

    [dependencies]
    uniffi = "0.20.0"
    uniffi_macros = "0.20.0"

    [build-dependencies]
    uniffi_build = { version = "0.20.0", features = ["builtin-bindgen"] }

   ```

1. create `./shared/shared.udl`

   ```txt
   namespace shared {
    u32 add(u32 left, u32 right);
   };

   ```

1. create `./shared/uniffi.toml`

   ```toml
   [bindings.kotlin]
   package_name = "redbadger.rmm.shared"
   cdylib_name = "shared"

   [bindings.swift]
   cdylib_name = "shared_ffi"
   omit_argument_labels = true

   ```

1. include the scaffolding in `shared/src/lib.rs`, and change types from `usize` to `u32`

   ```rust
   uniffi_macros::include_scaffolding!("shared");

   pub fn add(left: u32, right: u32) -> u32 {
     left + right
   }

   ```

1. create `./shared/build.rs`

   ```rust
   fn main() {
    uniffi_build::generate_scaffolding("./shared.udl").unwrap();
   }

   ```
