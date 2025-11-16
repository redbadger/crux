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

Let's call the app "SimpleCounter" and select "SwiftUI" for the interface and
"Swift" for the language. If you choose to create the app in the root folder of
your monorepo, then you might want to rename the folder it creates to "iOS".
Your repo's directory structure might now look something like this (some files
elided):

```txt
.
├── Cargo.lock
├── Cargo.toml
├── iOS
│  ├── SimpleCounter
│  │  ├── ContentView.swift
│  │  └── SimpleCounterApp.swift
│  └── SimpleCounter.xcodeproj
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
"${BUILD_DIR}/${CONFIGURATION}/uniffi-bindgen" generate "src/${INPUT_FILE_NAME}" --language swift --out-dir "${PROJECT_DIR}/generated"

```

We'll need to add the following as output files:

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE).swift
```

```txt
$(PROJECT_DIR)/generated/$(INPUT_FILE_BASE)FFI.h
```

Now go to the project settings, "**Build Phases, Compile Sources**", and add `/shared/src/shared.udl`
using the "add other" button, selecting "Create folder references".

You may also need to go to "**Build Settings, User Script Sandboxing**" and set this
to `No` to give the script permission to create files.

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

We will use [`cargo-xcode`](https://crates.io/crates/cargo-xcode) to generate an
Xcode project for our shared library, which we can add as a sub-project in
Xcode.

````admonish

Recent changes to `cargo-xcode` mean that we need to use version <=1.7.0 for
now.

If you don't have this already, you can install it in one of two ways:

1.  Globally, with `cargo install --force cargo-xcode --version 1.7.0`
2.  Locally, using
    [`cargo-run-bin`](https://github.com/dustinblackman/cargo-run-bin), after
    ensuring that your `Cargo.toml` has the following lines (see
    [The workspace and library manifests](../core.html#the-workspace-and-library-manifests)):

    ```toml
    [workspace.metadata.bin]
    cargo-xcode = { version = "=1.7.0" }
    ```

    Ensure you have `cargo-run-bin` (and optionally `cargo-binstall`) installed:

    ```bash
    cargo install cargo-run-bin cargo-binstall
    ```

    Then, in the root of your app:

    ```bash
    cargo bin --install # will be faster if `cargo-binstall` is installed
    cargo bin --sync-aliases # to use `cargo xcode` instead of `cargo bin xcode`
    ```

````

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

```admonish example
There is slightly more advanced
[example](https://github.com/redbadger/crux/tree/master/examples/counter) of an
iOS app in the Crux repository.

However, we will use the
[simple counter example](https://github.com/redbadger/crux/tree/master/examples/simple_counter),
which has `shared` and `shared_types` libraries that will work with the
following example code.
```

### Simple counter example

A simple app that increments, decrements and resets a counter.

#### Wrap the core to support capabilities

First, let's add some boilerplate code to wrap our core and handle the
capabilities that we are using. For this example, we only need to support the
`Render` capability, which triggers a render of the UI.

```admonish
This code that wraps the core only needs to be written once — it only grows when
we need to support additional capabilities.
```

Edit `iOS/SimpleCounter/core.swift` to look like the following. This code sends
our (UI-generated) events to the core, and handles any effects that the core
asks for. In this simple example, we aren't calling any HTTP APIs or handling
any side effects other than rendering the UI, so we just handle this render
effect by updating the published view model from the core.

```swift
{{#include ../../../../examples/simple_counter/iOS/SimpleCounter/core.swift}}
```

```admonish tip
That `switch` statement, above, is where you would handle any other effects that
your core might ask for. For example, if your core needs to make an HTTP
request, you would handle that here. To see an example of this, take a look at
the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter/iOS/CounterApp/core.swift)
in the Crux repository.
```

Edit `iOS/SimpleCounter/ContentView.swift` to look like the following:

```swift
{{#include ../../../../examples/simple_counter/iOS/SimpleCounter/ContentView.swift}}
```

And create `iOS/SimpleCounter/SimpleCounterApp.swift` to look like this:

```swift
{{#include ../../../../examples/simple_counter/iOS/SimpleCounter/SimpleCounterApp.swift}}
```

```admonish success
You should then be able to run the app in the simulator or on an iPhone, and it should look like this:

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
