# Shared core and types

These are the steps to set up the two crates forming the shared core – the core itself, and the shared types crate which does type generation for the foreign languages.

> _SHARP EDGE WARNING_: Most of these steps are going to be automated in future tooling, and published as crates. For now the set up is effectively a copy & paste from the example project.

## Install the tools

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

## Create the core crate

1. Create a new rust library ...

   ```sh
   cargo new --lib shared
   ```

1. Edit `./Cargo.toml` to add the new library to the Cargo workspace

   ```toml
   [workspace]
   members = ["shared"]
   ```

1. Edit `./shared/Cargo.toml`

   Note that the crate type:

   1. `"lib"` is the default rust library when linking into a rust binary, e.g. in the `web-yew` variant
   2. `"staticlib"` is a static library (`libshared.a`) for including in the Swift iOS app variant
   3. `"cdylib"` is a c-abi dynamic library (`libshared.so`) for use with JNA when included in the Kotlin Android app variant

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

   > _FIXME:_ update the above to include the framework crate. For now, you will have to copy it into your codebase or fetch from GitHub.

1. Create `./shared/src/shared.udl`

   ```txt
   namespace shared {
     sequence<u8> message([ByRef] sequence<u8> msg);
     sequence<u8> response([ByRef] sequence<u8> res);
     sequence<u8> view();
   };
   ```

1. Create `./shared/uniffi.toml`

   ```toml
   [bindings.kotlin]
   package_name = "com.redbadger.rmm.shared"
   cdylib_name = "shared"

   [bindings.swift]
   cdylib_name = "shared_ffi"
   omit_argument_labels = true
   ```

1. Create `./shared/build.rs`]

   ```rust
   fn main() {
       uniffi_build::generate_scaffolding("./src/shared.udl").unwrap();
   }
   ```

1. Include the scaffolding in `./shared/src/lib.rs`

   ```rust
    use lazy_static::lazy_static;

    use wasm_bindgen::prelude::wasm_bindgen;
    pub mod app;
    pub use app::*;

    uniffi_macros::include_scaffolding!("shared");

    lazy_static! {
        static ref CORE: AppCore<CatFacts> = Default::default();
    }

    # [wasm_bindgen]
    pub fn message(data: &[u8]) -> Vec<u8> {
        CORE.message(data)
    }

    # [wasm_bindgen]
    pub fn response(data: &[u8]) -> Vec<u8> {
        CORE.response(data)
    }

    # [wasm_bindgen]
    pub fn view() -> Vec<u8> {
        CORE.view()
    }
   ```

1. Create a basic app implementation in `./shared/src/app.rs`

   ```rust
   pub use rmm::*;
   use serde::{Deserialize, Serialize};

   pub struct Model {
       count: u8
   }

   # [derive(Serialize, Deserialize, Default)]
   pub struct ViewModel {
       pub count: u8,
   }

   # [derive(Serialize, Deserialize)]
   pub enum Message {
       Increment(u8),
       Decrement(u8),
   }

   impl App for CatFacts {
       type Message = Message;
       type Model = Model;
       type ViewModel = ViewModel;

       fn update(&self, msg: Message, model: &mut Model) -> Vec<Command<Message>> {
           match msg {
               Message::Increment(n) => {
                   model.count += n;

                   vec![Command::Render]
               },
               Message::Decrement(n) => {
                   model.count -= n;

                   vec![Command::Render]
               },
           }
       }

       fn view(&self, model: &Model) -> ViewModel {
           ViewModel {
               count: model.count
           }
       }
   }
   ```

1. Make sure everything builds OK

   ```sh
   cargo build
   ```

## Create the shared types crate

This crate serves as the container for type generation for the foreign languages.

1. Copy over the [shared_types](https://github.com/redbadger/rmm/tree/docs/shared_types/) folder from the example.

1. Edit the `build.rs` file and make sure to list your specific `Model`, `ViewModel` and `Message` types.

1. Make sure everything builds and foreign types get generated into the `generated` folder.

   ```sh
   cargo build -vv
   ```
