# iOS — Swift and SwiftUI — using XcodeGen

These are the steps to set up Xcode to build and run a simple iOS app that calls into a shared core.

```admonish tip
We think that using [XcodeGen](https://github.com/yonaskolb/XcodeGen) may be the simplest way to create an Xcode project to build and run a simple iOS app that calls into a shared core. If you'd rather set up Xcode manually, you can jump back to [iOS — Swift and SwiftUI — manual setup](./ios.md), otherwise read on.
```

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo — as described in [Shared core and types](./core.md) — and that you have built them using `cargo build`.
```

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

This generates an Xcode project for each crate in the workspace, but we're only interested in the one it creates in the `shared` directory. Don't open this generated project yet, it'll be included when we generate the Xcode project for our iOS app.

## Generate the Xcode project for our iOS app

```admonish
We will use [`XcodeGen`](https://github.com/yonaskolb/XcodeGen) to generate an Xcode project for our iOS app.

If you don't have this already, you can install it with `brew install xcodegen`.
```

Before we generate the Xcode project, we need to create some directories and a `project.yml` file:

```bash
mkdir -p iOS/CounterApp
cd iOS
touch project.yml
```

The `project.yml` file describes the Xcode project we want to generate. Here's one for the Counter example — you may want to adapt this for your own project:

```yaml
{{#include ../../../examples/counter/iOS/project.yml}}
```

Then we can generate the Xcode project:

```bash
xcodegen
```

This should create an `iOS/CounterApp/CounterApp.xcodeproj` project file, which we can open in Xcode. It should build OK, but we will need to add some code!

## Create some UI and run in the Simulator, or on an iPhone

### Hello World counter example

```admonish example
There are several [examples](https://github.com/redbadger/crux/tree/master/examples) of iOS apps in the Crux repository.

However, the simplest example is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world) — it only has `shared` and `shared_types` libraries, which will work with the following example code.
```

Create `iOS/CounterApp/ContentView.swift` to look like this:

```swift
{{#include ../../../examples/hello_world/iOS/CounterApp/ContentView.swift}}
```

And create `iOS/CounterApp/CounterAppApp.swift` to look like this:

```swift
{{#include ../../../examples/hello_world/iOS/CounterApp/CounterAppApp.swift}}
```

Run `xcodegen` again to update the Xcode project with these newly created source files (or add them manually in Xcode to the `CounterApp` group), and then open `iOS/CounterApp/CounterApp.xcodeproj` in Xcode. You might need to select the `CounterApp` scheme, and an appropriate simulator, in the drop-down at the top, before you build.

```admonish success
You should then be able to run the app in the simulator or on an iPhone, and it should look like this:

<p align="center"><img alt="hello world app" src="./hello_world_ios.webp"  width="300"></p>
```
