# iOS with Swift

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

To achieve this, we'll associate a script with files that match the pattern `*.udl` (this matches the interface definition file we created earlier), and then add our `shared.udl` file to the project.

Note that the script assumes we installed Uniffi with `cargo install uniffi`, as described earlier.

Add a build rule to process files that match the pattern `*.udl` with the following script (and also uncheck "Run once per architecture").

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

We'll need to add the following as output files:

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE).swift
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE)FFI.h
```

Now we can add `../shared/src/shared.udl` to "Compile Sources" (using the "add other" button), selecting "Copy items if needed" and "Create folder references".

## Configure the build

In "Build Settings" ...

1.  Add a "User-defined setting" called "`build_variant`", with a value of `debug` for Debug and `release` for Release
1.  Search for "bridging header", and add `generated/sharedFFI.h`, for any architecture/SDK, in both Debug and Release.
    If there isn't already a setting for "bridging header" you can add one (and then delete it) as per [this StackOverflow question](https://stackoverflow.com/questions/41787935/how-to-use-objective-c-bridging-header-in-a-swift-project/41788055#41788055)
1.  Search for "library search paths" and add some dummy values for debug and release.
    This will update the project file so you can search in it for `LIBRARY_SEARCH_PATHS` in the next step.

1.  Open `./iOS/iOs.xcodeproj/project.pbxproj` in a code editor and search for "LIBRARY_SEARCH_PATHS" (you should find 2 occurrences), and add the following ...

    1.  In the "debug" section

    ```txt
    "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios/debug";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios-sim/debug";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../target/x86_64-apple-ios/debug";
    ```

    1.  In the "release"" section

    ```txt
    "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios/release";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios-sim/release";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../target/x86_64-apple-ios/release";
    ```

1.  Create a script to build the Rust library (e.g. this script [`/examples/cat_facts/iOS/bin/compile-library.sh`](../examples/cat_facts/iOS/bin/compile-library.sh))
1.  In "Build phases", create or modify the following phases (you can drag them so that they match the order below) ...

    1.  Add a "New Run Script Phase" with the following script, and uncheck "Based on dependency analysis".
        You can rename it to something like "Build Rust library" by double clicking on the heading.

        ```sh
        cd "${PROJECT_DIR}"/../shared
        bash "${PROJECT_DIR}/bin/compile-library.sh" shared "$build_variant"
        ```

    1.  Test the build (which will still fail, but should create the `generated` directory)
    1.  Add a "Headers" section that includes `./iOS/generated/sharedFFI.h` as a "Public" header. Drag the
        "Build Rust library" phase that was added above, so that it appears above this new "Headers" phase.
    1.  Add `../target/debug/libshared.a` to the "Link Binary with Libraries" section (this is the wrong target, but the library search paths, which we set above, should resolve this.
        For more info see the blog post linked above ([this post](https://blog.mozilla.org/data/2022/01/31/this-week-in-glean-building-and-deploying-a-rust-library-on-ios/)))
