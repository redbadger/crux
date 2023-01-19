# iOS with Swift and SwiftUI

We want to make setting up xCode to work with Crux really easy. As time progresses we will try to simplify and automate as much as possible, but at the moment there is some manual configuration to do.

> This only needs doing once, so we hope it's not too much trouble, but in the future, we intend to provide some tooling to help with these set up activities. If you know of any better ways than those we describe below (e.g. how to do xCode project configuration from the command line), please either raise an issue (or a PR) at <https://github.com/redbadger/crux>.

## Create an iOS App

The first thing we need to do is create a new iOS app in xCode.

Let's call the app "iOS" and select "SwiftUI" for the interface and "Swift" for the language. If you choose to create the app in the root folder then your repo's directory structure might now look something like this (some files elided):

```txt
.
├── Cargo.lock
├── Cargo.toml
├── iOS
│  ├── HelloWorld
│  │  ├── ContentView.swift
│  │  └── HelloWorldApp.swift
│  └── HelloWorld.xcodeproj
│     └── project.pbxproj
├── shared
│  ├── build.rs
│  ├── Cargo.toml
│  ├── src
│  │  ├── hello_world.rs
│  │  ├── lib.rs
│  │  └── shared.udl
│  └── uniffi.toml
├── shared_types
│  ├── build.rs
│  ├── Cargo.toml
│  └── src
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
if [ "$ACTION" == "indexbuild" ]; then
   echo "Not building *.udl files during indexing."
   exit 0
fi

# Skip for preview builds
if [ "$ENABLE_PREVIEWS" = "YES" ]; then
   echo "Not building *.udl files during preview builds."
   exit 0
fi

# `swiftformat` is used by uniffi_bindgen, so update PATH if it was installed with homebrew
export PATH=${PATH}:/opt/homebrew/bin

cd "$INPUT_FILE_DIR/.."
"$HOME/.cargo/bin/uniffi-bindgen" generate "src/$INPUT_FILE_NAME" --language swift --out-dir "$PROJECT_DIR/generated"
```

We'll need to add the following as output files:

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE).swift
```

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE)FFI.h
```

Now go to "Build Phases" => "Compile Sources", and add `/shared/src/shared.udl` using the "add other" button, selecting "Copy items if needed" and "Create folder references".

Build the project (cmd-B), which will fail, but the above script should run successfully and the "generated" folder should contain the generated Swift types and C header files:

```bash
$ ls iOS/generated
shared.swift  sharedFFI.h  sharedFFI.modulemap
```

## Compile our Rust shared library

When we build our iOS app, we also want to build the Rust core as a static library so that it can be linked into the binary that we're going to ship. We do this with Cargo, specifying the relevant target.

Create a group called `bin` in your xCode project and add a shell script (called something like `rust_build.sh`) to it (don't forget to tick the box to ensure it targets our iOS app), with the following contents:

```bash
{{#include ../../../scripts/ios_build.sh}}
```

Then create a new "Build Phase" of type "Run Script" (called something like `Build Rust library` — you can rename by double-clicking) to call the script something like this:

```bash
cd "$PROJECT_DIR/../shared"
bash "$PROJECT_DIR/bin/rust_build.sh" shared
```

Uncheck "Based on dependency analysis".

You can drag this build phase up a bit (e.g. before "Compile Sources"), and test that it compiles the Rust library when you build your project.

## Link the Rust shared library into our iOS binary

Now that we have successfully compiled the share Rust library, we need to link it into the iOS binary. We need to tell xCode where to find the relevant static library based on which build configuration we have built for (`Debug` or `Release`).

This is a little convoluted, but this may be the easiest way to do this:

1.  In "Build Settings", search for "library search paths" and add a dummy string "XXXX" for debug and release (this will update the project file so you can search in it for `XXXX` in the next step).

1.  Open the project configuration file (`*.pbxproj`) in a code editor and search for "XXXX" (you should find 2 occurrences), and replace it with the following:

    1.  In the "Debug" section

    ```txt
    "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios/debug";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios-sim/debug";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../target/x86_64-apple-ios/debug";
    ```

    1.  In the "Release"" section

    ```txt
    "LIBRARY_SEARCH_PATHS[sdk=iphoneos*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios/release";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=arm64]" = "$(PROJECT_DIR)/../target/aarch64-apple-ios-sim/release";
    "LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*][arch=x86_64]" = "$(PROJECT_DIR)/../target/x86_64-apple-ios/release";
    ```

1.  In "Build Phases", add `/target/debug/libshared.a` to the "Link Binary with Libraries" section (this is the wrong target, but the library search paths, which we set above, should resolve this.
    For more info see the blog post linked above ([this post](https://blog.mozilla.org/data/2022/01/31/this-week-in-glean-building-and-deploying-a-rust-library-on-ios/)))

## Add the `Serde` package

In order to serialize data across the "bridge" we need to add the "Serde" package to our project. You can do this with `File -> Add Packages` and search for "Serde".

## Add the bridging header

In "Build Settings", search for "bridging header", and add `generated/sharedFFI.h`, for any architecture/SDK, i.e. in both Debug and Release.
If there isn't already a setting for "bridging header" you can add one (and then delete it) as per [this StackOverflow question](https://stackoverflow.com/questions/41787935/how-to-use-objective-c-bridging-header-in-a-swift-project/41788055#41788055)

## Add the Shared Types

In `File -> Add Files to iOS`, add `/shared_types/generated/swift/shared_types.swift`.

## Create some UI and run in the Simulator

### Hello World counter example

There are several [examples](https://github.com/redbadger/crux/tree/master/examples) of UI for iOS in the Crux repository. The simplest is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world), but this deliberately does not have an iOS example.

Edit `ContentView.swift` to look like this:

```swift
import Serde
import SwiftUI

enum Message {
    case message(Event)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(count: "")

    init() {
        update(msg: .message(.reset))
    }

    func update(msg: Message) {
        let reqs: [Request]

        switch msg {
        case let .message(m):
            reqs = try! [Request].bcsDeserialize(input: iOS.message(try! m.bcsSerialize()))
        }

        for req in reqs {
            switch req.effect {
            case .render(_): view = try! ViewModel.bcsDeserialize(input: iOS.view())
            }
        }
    }
}

struct ActionButton: View {
    var label: String
    var color: Color
    var action: () -> Void

    init(label: String, color: Color, action: @escaping () -> Void) {
        self.label = label
        self.color = color
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            Text(label)
                .fontWeight(.bold)
                .font(.body)
                .padding(EdgeInsets(top: 10, leading: 15, bottom: 10, trailing: 15))
                .background(color)
                .cornerRadius(10)
                .foregroundColor(.white)
                .padding()
        }
    }
}

struct ContentView: View {
    @ObservedObject var model: Model

    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            Text(model.view.count)
            HStack {
                ActionButton(label: "Reset", color: .red) {
                    model.update(msg: .message(.reset))
                }
                ActionButton(label: "Inc", color: .green) {
                    model.update(msg: .message(.increment))
                }
                ActionButton(label: "Dec", color: .yellow) {
                    model.update(msg: .message(.decrement))
                }
            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(model: Model())
    }
}
```

And edit `iosApp.swift` to look like this:

```swift
import SwiftUI

@main
struct iOSApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView(model: Model())
        }
    }
}
```

You should then be able to run the app in the simulator, and it should look like this:

<img alt="hello world app" src="./hello_world.webp"  width="300">
