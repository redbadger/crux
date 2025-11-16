# iOS — Swift and SwiftUI — using XcodeGen

These are the steps to set up Xcode to build and run a simple iOS app that calls
into a shared core.

```admonish tip
We think that using [XcodeGen](https://github.com/yonaskolb/XcodeGen) may be the simplest way to create an Xcode project to build and run a simple iOS app that calls into a shared core. If you'd rather set up Xcode manually, you can jump to [iOS — Swift and SwiftUI — manual setup](./manual.md), otherwise read on.
```

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo — as described in [Shared core and types](../core.md).
```

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
generated project yet, it'll be included when we generate the Xcode project for
our iOS app.

## Generate the Xcode project for our iOS app

```admonish
We will use [`XcodeGen`](https://github.com/yonaskolb/XcodeGen) to generate an Xcode project for our iOS app.

If you don't have this already, you can install it with `brew install xcodegen`.
```

Before we generate the Xcode project, we need to create some directories and a
`project.yml` file:

```bash
mkdir -p iOS/SimpleCounter
cd iOS
touch project.yml
```

The `project.yml` file describes the Xcode project we want to generate. Here's
one for the SimpleCounter example — you may want to adapt this for your own
project:

```yaml
{{#include ../../../../examples/simple_counter/iOS/project.yml}}
```

Then we can generate the Xcode project:

```bash
xcodegen
```

This should create an `iOS/SimpleCounter/SimpleCounter.xcodeproj` project file,
which we can open in Xcode. It should build OK, but we will need to add some
code!

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

Run `xcodegen` again to update the Xcode project with these newly created source
files (or add them manually in Xcode to the `SimpleCounter` group), and then
open `iOS/SimpleCounter/SimpleCounter.xcodeproj` in Xcode. You might need to
select the `SimpleCounter` scheme, and an appropriate simulator, in the
drop-down at the top, before you build.

```admonish success
You should then be able to run the app in the simulator or on an iPhone, and it should look like this:

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
