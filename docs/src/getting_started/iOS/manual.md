# iOS — Swift and SwiftUI — manual setup

These are the steps to set up Xcode to build and run a simple iOS app that calls
into a shared core.

```admonish warning
We recommend setting up Xcode with XcodeGen as described in the
[previous section](./with_xcodegen.md). It is the simplest way to create an Xcode
project to build and run a simple iOS app that calls into a shared core. However,
if you want to set up Xcode manually then read on.
```

```admonish
This walk-through assumes you have already added the `shared` and `shared_types`
libraries to your repo — as described in [Shared core and types](../core.md)
— and that you have built them using `cargo build`.
```

## Create an iOS App

The first thing we need to do is create a new iOS app in Xcode.

Let's call the app "CounterApp" and select "SwiftUI" for the interface and
"Swift" for the language. If you choose to create the app in the root folder of
your monorepo, then you might want to rename the folder it creates to "iOS".
Your repo's directory structure might now look something like this (some files
elided):

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
│  │  ├── counter.rs
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

We want UniFFI to create the Swift bindings and the C headers for our shared
library, and store them in a directory called `generated`.

To achieve this, we'll associate a script with files that match the pattern
`*.udl` (this will catch the interface definition file we created earlier), and
then add our `shared.udl` file to the project.

Note that our shared library generates the `uniffi-bindgen` binary (as explained
on the page ["Shared core and types"](../core.md)) that the script relies on, so
make sure you have built it already, using `cargo build`.

In "**Build Rules**", add a rule to process files that match the pattern `*.udl`
with the following script (and also uncheck "**Run once per architecture**").

```bash
#!/bin/bash
set -e

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

cd "${INPUT_FILE_DIR}/.."
"${BUILD_DIR}/debug/uniffi-bindgen" generate "src/${INPUT_FILE_NAME}" --language swift --out-dir "${PROJECT_DIR}/generated"

```

We'll need to add the following as output files:

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE).swift
```

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE)FFI.h
```

Now go to "**Build Phases, Compile Sources**", and add `/shared/src/shared.udl`
using the "add other" button, selecting "Create folder references".

Build the project (cmd-B), which will fail, but the above script should run
successfully and the "generated" folder should contain the generated Swift types
and C header files:

```bash
$ ls iOS/generated
shared.swift  sharedFFI.h  sharedFFI.modulemap
```

### Add the bridging header

In "**Build Settings**", search for "bridging header", and add
`generated/sharedFFI.h`, for any architecture/SDK, i.e. in both Debug and
Release. If there isn't already a setting for "bridging header" you can add one
(and then delete it) as per
[this StackOverflow question](https://stackoverflow.com/questions/41787935/how-to-use-objective-c-bridging-header-in-a-swift-project/41788055#41788055)

## Compile our Rust shared library

When we build our iOS app, we also want to build the Rust core as a static
library so that it can be linked into the binary that we're going to ship.

```admonish
We will use [`cargo-xcode`](https://crates.io/crates/cargo-xcode) to generate an Xcode project for our shared library, which we can add as a sub-project in Xcode.

If you don't have this already, you can install it with `cargo install cargo-xcode`.
```

Let's generate the sub-project:

```bash
cargo xcode
```

This generates an Xcode project for each crate in the workspace, but we're only
interested in the one it creates in the `shared` directory. Don't open this
generated project yet.

Using Finder, drag the `shared/shared.xcodeproj` folder under the Xcode project
root.

Then, in the "**Build Phases, Link Binary with Libraries**" section, add the
`libshared_static.a` library (you should be able to navigate to it as
`Workspace -> shared -> libshared_static.a`)

## Add the Shared Types

Using Finder, drag the `shared_types/generated/swift/SharedTypes` folder under
the Xcode project root.

Then, in the "**Build Phases, Link Binary with Libraries**" section, add the
`SharedTypes` library (you should be able to navigate to it as
`Workspace -> SharedTypes -> SharedTypes`)

## Create some UI and run in the Simulator, or on an iPhone

### Simple counter example

```admonish example
There are several [examples](https://github.com/redbadger/crux/tree/master/examples) of iOS apps in the Crux repository.

We will use the [simple counter example](https://github.com/redbadger/crux/tree/master/examples/simple_counter), which has `shared` and `shared_types` libraries that will work with the following example code.
```

Edit `ContentView.swift` to look like this:

```swift
{{#include ../../../../examples/simple_counter/iOS/CounterApp/ContentView.swift}}
```

And edit `CounterAppApp.swift` to look like this:

```swift
{{#include ../../../../examples/simple_counter/iOS/CounterApp/CounterAppApp.swift}}
```

```admonish success
You should then be able to run the app in the simulator or on an iPhone, and it should look like this:

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
