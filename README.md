# Rust Multi-platform Mobile (RMM)

This repo is a "hello world" example of using a single shared library, written in rust, in an Android app, an iOS app, and a web application.

For mobile, it uses [`uniffi`](https://github.com/mozilla/uniffi-rs) from Mozilla, which generates all the necessary bindings and marshalls FFI calls between kotlin/swift and rust.

For web it uses [`yew`](https://yew.rs/).

## Shared rust library

1. Make sure you have the following rust targets installed (e.g. `rustup target add aarch64-apple-ios`)

   ```txt
   aarch64-apple-darwin
   aarch64-apple-ios
   aarch64-apple-ios-sim
   aarch64-linux-android
   wasm32-unknown-unknown
   x86_64-apple-ios
   ```

1. Create a new rust library ...

   ```sh
   cargo new --lib shared
   ```

1. Edit [`./shared/Cargo.toml`](./shared/Cargo.toml).
   Note that the crate type:

   1. `"lib"` is the default rust library for use when linking into a rust binary, e.g. for WebAssembly in the web variant
   1. `"staticlib"` is a static library (`libshared.a`) for including in the Swift iOS app variant
   1. `"cdylib"` is a c-abi dynamic library (`libshared.so`) for use with JNA when included in the Kotlin Android app variant

   ```toml
   [lib]
   crate-type = ["lib", "staticlib", "cdylib"]
   name = "shared"

   [dependencies]
   uniffi = "0.20.0"
   uniffi_macros = "0.20.0"

   [build-dependencies]
   uniffi_build = { version = "0.20.0", features = ["builtin-bindgen"] }

   ```

1. Create [`./shared/shared.udl`](./shared/shared.udl) ...

   ```txt
   namespace shared {
     u32 add(u32 left, u32 right);
   };

   ```

1. Create [`./shared/uniffi.toml`](./shared/uniffi.toml) ...

   ```toml
   [bindings.kotlin]
   package_name = "redbadger.rmm.shared"
   cdylib_name = "shared"

   [bindings.swift]
   cdylib_name = "shared_ffi"
   omit_argument_labels = true

   ```

1. Create [`./shared/build.rs`](./shared/build.rs) ...

   ```rust
   fn main() {
        uniffi_build::generate_scaffolding("./shared.udl").unwrap();
   }

   ```

1. Include the scaffolding in [`./shared/src/lib.rs`](./shared/src/lib.rs), and change types from `usize` to `u32` ...

   ```rust
   uniffi_macros::include_scaffolding!("shared");

   pub fn add(left: u32, right: u32) -> u32 {
       left + right
   }

   ```

1. Make sure everything builds OK ...
   ```sh
   (cd shared && cargo build)
   ```

## Android App

1. Create a Kotlin App in Android Studio (e.g. "Basic Activity (Material3)" at `/Android`)

1. Add a Kotlin Android Library (`aar`) — you can find more details on how to do this [here](https://developer.android.com/studio/projects/android-library), but in a nutshell ...

   1. go to File -> New -> New Module...
   1. choose "Android Library"
   1. Call it e.g. `shared`
   1. Package name must match that in [`./shared/uniffi.toml`](./shared/uniffi.toml), e.g. `redbadger.rmm.shared`

1. Add the `shared` library as a dependency of `app`
   1. either...
      1. go to File -> Project Structure...
      1. choose "Dependencies"
      1. choose `app` and use the `+` to add a "Module dependency"
      1. select the `shared` library
   1. or...
      1. add this line to the `dependencies` section in [`./Android/app/build.gradle`](./Android/app/build.gradle)
         ```groovy
         implementation project(path: ':shared')
         ```
1. Generate the Kotlin source code:

   ```bash
    (cd shared && uniffi-bindgen generate ./shared.udl --language kotlin)
   ```

1. SymLink this into the share `aar` library

   ```bash
   ( \
      cd Android/shared/src/main/java && \
      rm -rf redbadger && \
      ln -s ../../../../../shared/redbadger redbadger \
   )
   ```

1. Mozilla has a rust gradle plugin for android [here](https://github.com/mozilla/rust-android-gradle). Add the plugin to `./Android/build.gradle`, and sync ...

   ```groovy
   plugins {
      id "org.mozilla.rust-android-gradle.rust-android" version "0.9.3"
   }
   ```

1. We are also using [Java Native Access (JNA)](https://github.com/java-native-access/jna). Add the following to `./Android/shared/build.gradle`, and sync ...

   ```groovy
   plugins {
      ...
      id 'org.mozilla.rust-android-gradle.rust-android'
   }
   android {
      namespace 'redbadger.rmm.shared'
      ...
      ndkVersion '25.1.8937393'
   }

   dependencies {
      implementation "net.java.dev.jna:jna:5.12.1@aar"
      ...
   }

   apply plugin: 'org.mozilla.rust-android-gradle.rust-android'

   cargo {
      module  = "../../shared"
      libname = "shared"
      targets = ["arm64"]
   }

   afterEvaluate {
      // The `cargoBuild` task isn't available until after evaluation.
      android.libraryVariants.all { variant ->
         def productFlavor = ""
         variant.productFlavors.each {
               productFlavor += "${it.name.capitalize()}"
         }
         def buildType = "${variant.buildType.name.capitalize()}"
         tasks["generate${productFlavor}${buildType}Assets"].dependsOn(tasks["cargoBuild"])
      }
   }
   ```

1. Run "Build -> Make project" to make sure that everything compiles (including the shared rust library) — you should be able to see the library object file ...

   ```sh
   ls Android/shared/build/rustJniLibs/android/arm64-v8a
   libshared.so
   ```

1. Try calling into the rust library from the Android app, for example ...
   1. open `Android/app/src/main/java/com/example/android/MainActivity.kt`
   1. add `import redbadger.rmm.shared.add`
   1. call the `add` function somewhere, e.g. on line 35...
      ```kotlin
      binding.fab.setOnClickListener { view ->
         Snackbar.make(view, "1 + 2 = " + add(1u, 2u), Snackbar.LENGTH_LONG)
               .setAnchorView(R.id.fab)
               .setAction("Action", null).show()
      }
      ```
   1. run the app in a simulator and check that the shared function is called (e.g. click on the snackbar button)

## iOS App

(adapted, for UniFFI, from [this post](https://blog.mozilla.org/data/2022/01/31/this-week-in-glean-building-and-deploying-a-rust-library-on-ios/) by Jan-Erik Rediger, with thanks.)

1. Open xCode and create a new iOS app (e.g. called `iOS` with organization `com.redbadger`)

1. Add a build rule to process files that match the pattern `*.udl` with the following script.
   This will use Uniffi to create the swift bindings and the C headers in a `generated` directory.
   Uncheck "Run once per architecture" ...

   ```bash
   # Skip during indexing phase in XCode 13+
   if [ $ACTION == "indexbuild" ]; then
      echo "Not building *.udl files during indexing."
      exit 0
   fi

   # Skip for preview builds
   if [ "${ENABLE_PREVIEWS}" = "YES" ]; then
      echo "Not building *.udl files during preview builds."
      exit 0
   fi

   cd "$INPUT_FILE_DIR"/.. && "$HOME"/.cargo/bin/uniffi-bindgen generate src/"$INPUT_FILE_NAME" --language swift --out-dir "$PROJECT_DIR/generated"
   ```

   Also add the following as output files:

   ```txt
   $(PROJECT_DIR)/generated/$(INPUT_FILE_BASE).swift
   $(PROJECT_DIR)/generated/$(INPUT_FILE_BASE)FFI.h
   ```

1. In "Build Settings" ...

   1. add a "User-defined setting" called "`build_variant`", with a value of `debug` for Debug and `release` for Release
   1. search for "bridging header", and add `generated/sharedFFI.h`, for any architecture/SDK, in both Debug and Release
   1. search for "library search paths" and add some dummy values for debug and release. This will update the project file so you can search in it for `LIBRARY_SEARCH_PATHS` in the next step.

1. Open `./iOS/iOs.xcodeproj/project.pbxproj` in a code editor and search for "LIBRARY_SEARCH_PATHS" (you should find 2 occurrences), and add the following ...

   1. in the "debug" section

   ```txt
   "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../shared/target/aarch64-apple-ios/debug";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../shared/target/aarch64-apple-ios-sim/debug";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../shared/target/x86_64-apple-ios/debug";
   ```

   1. in the "release"" section

   ```txt
   "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../shared/target/aarch64-apple-ios/release";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../shared/target/aarch64-apple-ios-sim/release";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../shared/target/x86_64-apple-ios/release";
   ```

1. Create a script to build the rust library (e.g. this script [`./iOS/bin/compile-library.sh`](./iOS/bin/compile-library.sh))
1. Test the build (which will still fail, but should create the `generated` directory)
1. In "Build phases", create or modify the following phases (you can drag them so that they match the order below) ...

   1. add a "New Run Script Phase" with the following script, and uncheck "Based on dependency analysis". You can rename it to something like "Build Rust library" by double clicking on the heading. ...

      ```sh
      cd "${PROJECT_DIR}"/../shared
      bash "${PROJECT_DIR}/bin/compile-library.sh" shared "$build_variant"
      ```

   1. add `./shared/src/shared.udl` to "Compile Sources" (using the "add other" button). Select"Copy items if needed" and "Create folder references"
   1. add a "Headers" section that includes `./iOS/generated/sharedFFI.h` as a "Public" header
   1. add `./shared/target/debug/libshared.a` to the "Link Binary with Libraries" section (this is the wrong target, but the library search paths, which we set above, should resolve this, for more info see the blog post linked above ([this post](https://blog.mozilla.org/data/2022/01/31/this-week-in-glean-building-and-deploying-a-rust-library-on-ios/)))

1. Test it out, by calling the `add` function, e.g. by changing `./iOS/iOS/ContentView.swift` to look like this:

   ```swift
   import SwiftUI

   struct ContentView: View {
      var body: some View {
         VStack {
               Image(systemName: "globe")
                  .imageScale(.large)
                  .foregroundColor(.accentColor)
               Text("1 + 2 = \(add(1, 2))")
         }
         .padding()
      }
   }

   struct ContentView_Previews: PreviewProvider {
      static var previews: some View {
         ContentView()
      }
   }
   ```

## Web

1. Install [`trunk`](https://github.com/thedodd/trunk)
1. Create a new rust binary ...

   ```sh
   cargo new web
   ```

1. Add [`yew`](https://yew.rs/), and the shared library, as dependencies in [`./web/Cargo.toml`](./web/Cargo.toml) ...

   ```toml
   [dependencies]
   yew = "0.19.3"
   shared = { path = "../shared" }
   ```

1. Add [`./web/index.html`](./web/index.html) ...

   ```html
   <!DOCTYPE html>
   <html>
     <head>
       <meta charset="utf-8" />
       <title>Yew App</title>
     </head>
   </html>
   ```

1. Edit [`./web/src/main.rs`](./web/src/main.rs), for example ...

   ```rust
   use shared::add;
   use yew::prelude::*;

   #[function_component(HelloWorld)]
   fn hello_world() -> Html {
      html! {
         <p>{"1 + 2 = "}{add(1, 2)}</p>
      }
   }

   fn main() {
      yew::start_app::<HelloWorld>();
   }
   ```

1. Build and serve the web page ...

   ```sh
   cd ./web
   trunk serve
   ```
