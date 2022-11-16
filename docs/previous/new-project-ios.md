# iOS App

[Table of Contents](./new-project.md)

(Adapted for UniFFI from [this post](https://blog.mozilla.org/data/2022/01/31/this-week-in-glean-building-and-deploying-a-rust-library-on-ios/) by Jan-Erik Rediger, with thanks)

1. Open xCode and create a new iOS app (e.g. called `iOS` with organization `com.redbadger`)

1. Add a build rule to process files that match the pattern `*.udl` with the following script.
   This will use UniFFI to create the Swift bindings and the C headers in a `generated` directory.

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

   1. Add a "User-defined setting" called "`build_variant`", with a value of `debug` for Debug and `release` for Release
   1. Search for "bridging header", and add `generated/sharedFFI.h`, for any architecture/SDK, in both Debug and Release.
      If there isn't already a setting for "bridging header" you can add one (and then delete it) as per [this StackOverflow question](https://stackoverflow.com/questions/41787935/how-to-use-objective-c-bridging-header-in-a-swift-project/41788055#41788055)
   1. Search for "library search paths" and add some dummy values for debug and release.
      This will update the project file so you can search in it for `LIBRARY_SEARCH_PATHS` in the next step.

1. Open `./iOS/iOs.xcodeproj/project.pbxproj` in a code editor and search for "LIBRARY_SEARCH_PATHS" (you should find 2 occurrences), and add the following ...

   1. In the "debug" section

   ```txt
   "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios/debug";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios-sim/debug";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../target/x86_64-apple-ios/debug";
   ```

   1. In the "release"" section

   ```txt
   "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios/release";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios-sim/release";
   "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../target/x86_64-apple-ios/release";
   ```

1. Create a script to build the Rust library (e.g. this script [`./iOS/bin/compile-library.sh`](../iOS/bin/compile-library.sh))
1. Test the build (which will still fail, but should create the `generated` directory)
1. In "Build phases", create or modify the following phases (you can drag them so that they match the order below) ...

   1. Add a "New Run Script Phase" with the following script, and uncheck "Based on dependency analysis".
      You can rename it to something like "Build Rust library" by double clicking on the heading.

      ```sh
      cd "${PROJECT_DIR}"/../shared
      bash "${PROJECT_DIR}/bin/compile-library.sh" shared "$build_variant"
      ```

   1. Add `./shared/src/shared.udl` to "Compile Sources" (using the "add other" button).
      Select "Copy items if needed" and "Create folder references"
   1. Add a "Headers" section that includes `./iOS/generated/sharedFFI.h` as a "Public" header
   1. Add `./target/debug/libshared.a` to the "Link Binary with Libraries" section (this is the wrong target, but the library search paths, which we set above, should resolve this.
      For more info see the blog post linked above ([this post](https://blog.mozilla.org/data/2022/01/31/this-week-in-glean-building-and-deploying-a-rust-library-on-ios/)))

1. Add a `class` for the callback to get Platform details ...

   ```swift
   class GetPlatform: Platform {
       func get() -> String {
           return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
       }
   }
   ```

1. Call the `addForPlatform` function, e.g. in a Text UI component ...

   ```swift
   Text(try! addForPlatform(1, 2, GetPlatform()))
   ```
