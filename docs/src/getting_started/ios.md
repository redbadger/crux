# iOS with Swift

We want to make setting up xCode to work with Crux really easy. We will probably build some tooling to help with this, but at the moment there is some manual configuration of xCode to do.

This only needs doing once, so we hope it's not too much trouble, but in the future, we intend to provide some tooling to help with these set up activities. If you know of any better ways than those we describe below (e.g. how to do xCode project configuration form the command line), please either raise an issue or a PR at <https://github.com/redbadger/crux>.

## Create an iOS App

The first thing we want to do is create a new iOS app in xCode.

Choose a suitable name and organization for the App, select "SwiftUI" for the interface, and "Swift" for the language. On the next screen choose a new directory (e.g. `/iOS`) in the monorepo for this new project.

If you called your app "HelloWorld", then your repo's directory structure might now look like this (some files elided):

```txt
.
├── Cargo.lock
├── Cargo.toml
├── iOS
│  └── HelloWorld
│     ├── HelloWorld
│     │  ├── ContentView.swift
│     │  └── HelloWorldApp.swift
│     └── HelloWorld.xcodeproj
├── shared
│  ├── Cargo.toml
│  └── src
│     ├── hello_world.rs
│     └── lib.rs
└── target
```

## Generate FFI bindings

We want UniFFI to create the Swift bindings and the C headers for our shared library, and store them in a directory called `generated`.

To achieve this, we'll associate a script with files that match the pattern `*.udl` (this will catch the interface definition file we created earlier), and then add our `shared.udl` file to the project.

Note that the script assumes we installed Uniffi with `cargo install uniffi`, as described earlier.

In "Build Rules", add a rule to process files that match the pattern `*.udl` with the following script (and also uncheck "Run once per architecture").

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

# `swiftformat` is used by uniffi_bindgen, so update PATH if it was installed with homebrew
export PATH=${PATH}:/opt/homebrew/bin

cd "$INPUT_FILE_DIR/.."
"$HOME"/.cargo/bin/uniffi-bindgen generate src/"$INPUT_FILE_NAME" --language swift --out-dir "$PROJECT_DIR/generated"
```

We'll need to add the following as output files:

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE).swift
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE)FFI.h
```

Now we can add `/shared/src/shared.udl` to "Compile Sources" (using the "add other" button), selecting "Copy items if needed" and "Create folder references".

## Compile our Rust shared library

When we build our iOS app, we also want to build the Rust core as a static library so that it can be linked into the binary that we're going to ship. We do this with Cargo, specifying the relevant target.

Create a shell script in your xCode project (called something like `/bin/rust_build.sh`) and add the following contents:

```bash
{{#include ../../../scripts/ios_build.sh}}
```

Then create a new "Build Phase" (called something like `Build Rust library` — you can rename by double-clicking) to call the script something like this:

```bash
cd "$PROJECT_DIR/../shared"
bash "$PROJECT_DIR/bin/rust_build.sh" shared
```

You can drag this build phase up a bit (e.g. before "Compile Sources"), and test that it compiles the Rust library when you build your project.

## Link the Rust shared library into our iOS binary

Now that we have successfully compiled the share Rust library, we need to link it into the iOS binary. We need to tell xCode where to find the relevant static library based on which build configuration we have built for (`Debug` or `Release`).

This is a little convoluted, but this may be the easiest way to do this:

1.  In "Build Settings", search for "library search paths" and add a dummy string "XXXX" for debug and release (this will update the project file so you can search in it for `XXXX` in the next step).

1.  Open the project configuration file (`*.pbxproj`) in a code editor and search for "XXXX" (you should find 2 occurrences), and replace it with the following:

    1.  In the "Debug" section

    ```txt
    "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../../target/aarch64-apple-ios/debug";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../../target/aarch64-apple-ios-sim/debug";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../../target/x86_64-apple-ios/debug";
    ```

    1.  In the "Release"" section

    ```txt
    "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../../target/aarch64-apple-ios/release";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../../target/aarch64-apple-ios-sim/release";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../../target/x86_64-apple-ios/release";
    ```

1.  In "Build Phases", add `/target/debug/libshared.a` to the "Link Binary with Libraries" section (this is the wrong target, but the library search paths, which we set above, should resolve this.
    For more info see the blog post linked above ([this post](https://blog.mozilla.org/data/2022/01/31/this-week-in-glean-building-and-deploying-a-rust-library-on-ios/)))

## Add the bridging header

1.  In "Build Settings", search for "bridging header", and add `generated/sharedFFI.h`, for any architecture/SDK, i.e. in both Debug and Release.
    If there isn't already a setting for "bridging header" you can add one (and then delete it) as per [this StackOverflow question](https://stackoverflow.com/questions/41787935/how-to-use-objective-c-bridging-header-in-a-swift-project/41788055#41788055)

1.  In "Build Phases", add a "Headers" section that includes `generated/sharedFFI.h` as a "Public" header. Drag the
    "Build Rust library" phase that was added above, so that it appears above this new "Headers" phase.
