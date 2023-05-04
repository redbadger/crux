# iOS — Swift and SwiftUI

These are the steps to set up Xcode to build and run a simple iOS app that calls into a shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](./core.md).
```

```admonish warning title="Sharp edge"
We want to make setting up Xcode to work with Crux really easy. As time progresses we will try to simplify and automate as much as possible, but at the moment there is some manual configuration to do. This only needs doing once, so we hope it's not too much trouble. If you know of any better ways than those we describe below (e.g. how to do Xcode project configuration from the command line), please either raise an issue (or a PR) at <https://github.com/redbadger/crux>.
```

## Create an iOS App

The first thing we need to do is create a new iOS app in Xcode.

Let's call the app "CounterApp" and select "SwiftUI" for the interface and "Swift" for the language. If you choose to create the app in the root folder of your monorepo, then you might want to rename the folder it creates to "iOS". Your repo's directory structure might now look something like this (some files elided):

```txt
.
├── Cargo.lock
├── Cargo.toml
├── iOS
│  ├── CounterApp
│  │  ├── ContentView.swift
│  │  └── CounterAppApp.swift
│  └── CounterApp.xcodeproj
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

Note that our shared library generates the `uniffi-bindgen` binary (as explained on the page ["Shared core and types"](./core.md)) that the script relies on, so make sure you have built it already, using `cargo build`.

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

# note, for now, run a cargo build manually to ensure the binary exists for this step
cd "$INPUT_FILE_DIR"/.. && "$PROJECT_DIR/../target/debug/uniffi-bindgen" generate src/"$INPUT_FILE_NAME" --language swift --out-dir "$PROJECT_DIR/generated"
```

We'll need to add the following as output files:

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE).swift
```

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE)FFI.h
```

Now go to "Build Phases" => "Compile Sources", and add `/shared/src/shared.udl` using the "add other" button, selecting "Create folder references".

Build the project (cmd-B), which will fail, but the above script should run successfully and the "generated" folder should contain the generated Swift types and C header files:

```bash
$ ls iOS/generated
shared.swift  sharedFFI.h  sharedFFI.modulemap
```

### Add the bridging header

In "Build Settings", search for "bridging header", and add `generated/sharedFFI.h`, for any architecture/SDK, i.e. in both Debug and Release.
If there isn't already a setting for "bridging header" you can add one (and then delete it) as per [this StackOverflow question](https://stackoverflow.com/questions/41787935/how-to-use-objective-c-bridging-header-in-a-swift-project/41788055#41788055)


## Compile our Rust shared library

When we build our iOS app, we also want to build the Rust core as a static library so that it can be linked into the binary that we're going to ship.

```admonish
We will use [`cargo-xcode`](https://crates.io/crates/cargo-xcode) to generate an Xcode project for our shared library, which we can add as a sub-project in Xcode.

If you don't have this already, you can install it with `cargo install cargo-xcode`.
```

Let's generate the sub-project:

```bash
cargo xcode
```

This generates an Xcode project for each crate in the workspace, but we're only interested in the one it creates in the `shared` directory. Don't open this generated project yet.

Using Finder, drag the `shared/shared.xcodeproj` folder under the Xcode project root.

Then, in "Build Phases", add the static library to the "Link Binary with Libraries" section (you should be able to navigate to it under `Workspace -> shared -> libshared_static.a`)

## Add the Shared Types

In `File -> Add Files to "CounterApp"`, add `/shared_types/generated/swift/shared_types.swift`.

## Add the `Serde` package

In order to serialize data across the "bridge" we need to add the [`Serde` package](https://github.com/starcoin-sdk/Serde.swift) to our project. You can do this with `File -> Add Packages` and search for "https://github.com/starcoin-sdk/Serde.swift".

## Create some UI and run in the Simulator, or on an iPhone

### Hello World counter example

```admonish example
There are several [examples](https://github.com/redbadger/crux/tree/master/examples) of iOS apps in the Crux repository.

However, the simplest example is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world) — it only has `shared` and `shared_types` libraries, which will work with the following example code.
```

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
            reqs = try! [Request].bcsDeserialize(input: CounterApp.processEvent(try! m.bcsSerialize()))
        }

        for req in reqs {
            switch req.effect {
            case .render(_): view = try! ViewModel.bcsDeserialize(input: CounterApp.view())
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

And edit `CounterAppApp.swift` to look like this:

```swift
import SwiftUI

@main
struct CounterAppApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView(model: Model())
        }
    }
}
```

```admonish success
You should then be able to run the app in the simulator or on an iPhone, and it should look like this:

<p align="center"><img alt="hello world app" src="./hello_world_ios.webp"  width="300"></p>
```
