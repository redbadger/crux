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
   wasm-bindgen = "0.2.83"
   lazy_static = "1.4.0"
   crux_core = "0.2.0"
   serde = { version = "1.0.147", features = ["derive"] }
   bincode = "1.3.3"

   [build-dependencies]
   uniffi_build = { version = "0.21.0", features = ["builtin-bindgen"] }
   ```

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

1. Include the scaffolding in `./shared/src/lib.rs`

   ```rust
    pub mod app;

    use lazy_static::lazy_static;
    use wasm_bindgen::prelude::wasm_bindgen;

    use crux_core::Core;
    pub use crux_core::Request;
    pub use crux_http as http;

    pub use app::*;

    uniffi_macros::include_scaffolding!("shared");

    lazy_static! {
        static ref CORE: Core<Effect, App> = Core::new::<Capabilities>();
    }

    # [wasm_bindgen]
    pub fn message(data: &[u8]) -> Vec<u8> {
        CORE.message(data)
    }

    # [wasm_bindgen]
    pub fn response(uuid: &[u8], data: &[u8]) -> Vec<u8> {
        CORE.response(uuid, data)
    }

    # [wasm_bindgen]
    pub fn view() -> Vec<u8> {
        CORE.view()
    }
   ```

2. Create a basic app implementation in `./shared/src/app.rs`

   ```rust
   use crux_core::render::Render;
   use crux_macros::Effect;
   use serde::{Deserialize, Serialize};
   
   #[derive(Default)]
   pub struct App;
   
   impl crux_core::App for App {
       type Model = Model;
       type Event = Event;
       type ViewModel = ViewModel;
       type Capabilities = Capabilities;
   
       fn update(&self, msg: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
           match msg {
               Event::Increment => {
                   model.count += 1;
                   caps.render.render();
               },
               Event::Decrement => {
                   model.count -= 1;
                   caps.render.render();
               }
           }
       }
   
       fn view(&self, model: &Self::Model) -> Self::ViewModel {
           model.into()
       }
   }
   
   #[derive(Default)]
   pub struct Model {
       count: isize,
   }
   
   #[derive(Serialize, Deserialize)]
   pub struct ViewModel {
       pub text: String,
   }
   
   impl From<&Model> for ViewModel {
       fn from(model: &Model) -> Self {
           Self {
               text: model.count.value.to_string() + &suffix,
           }
       }
   }
   
   #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
   pub enum Event {
       Increment,
       Decrement,
   }
   
   #[derive(Effect)]
   pub struct Capabilities {
       pub render: Render<Event>,
   }
   ```

3. Make sure everything builds OK

   ```sh
   cargo build
   ```

## Create the shared types crate

This crate serves as the container for type generation for the foreign languages.

1. Copy over the [shared_types](https://github.com/redbadger/crux/tree/master/examples/counter/shared_types) folder from the counter example.

1. Edit the `build.rs` file and make sure to only list types you need.

1. Make sure everything builds and foreign types get generated into the `generated` folder.
   
   ```sh
   cargo build -vv
   ```

You should now be ready to set up [iOS](ios.md), [Android](android.md), [web](web_react.md), or [WebAssembly](web_yew.md) specific builds.
