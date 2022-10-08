# Rust Multi-platform Mobile (RMM)

## Shared rust library

1. Create a new rust library ...

   ```sh
   cargo new --lib shared
   ```

1. Edit `./shared/Cargo.toml` ...

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

1. Create `./shared/shared.udl` ...

   ```txt
   namespace shared {
     u32 add(u32 left, u32 right);
   };

   ```

1. Create `./shared/uniffi.toml` ...

   ```toml
   [bindings.kotlin]
   package_name = "redbadger.rmm.shared"
   cdylib_name = "shared"

   [bindings.swift]
   cdylib_name = "shared_ffi"
   omit_argument_labels = true

   ```

1. Create `./shared/build.rs` ...

   ```rust
   fn main() {
        uniffi_build::generate_scaffolding("./shared.udl").unwrap();
   }

   ```

1. Include the scaffolding in `./shared/src/lib.rs`, and change types from `usize` to `u32` ...

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
   1. Package name must match that in `./shared/uniffi.toml`, e.g. `redbadger.rmm.shared`
1. Add the `shared` library as a dependency of `app`
   1. either...
      1. go to File -> Project Structure...
      1. choose "Dependencies"
      1. choose `app` and use the `+` to add a "Module dependency"
      1. select the `shared` library
   1. or...
      1. add this line to the `dependencies` section in `./Android/app/build.gradle`
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
