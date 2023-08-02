# Web — Rust and Leptos

These are the steps to set up and run a simple Rust Web app that calls into a
shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](../core.md).
```

```admonish info
There are many frameworks available for writing Web applications in Rust. Here we're choosing [Leptos](https://leptos.dev/) for this walk-through as a way to demonstrate how Crux can work with web frameworks that use fine-grained reactivity rather than the conceptual full re-rendering of React. However, a similar setup would work for other frameworks that compile to WebAssembly.
```

## Create a Leptos App

Our Leptos app is just a new Rust project, which we can create with Cargo. For
this example we'll call it `web-leptos`.

```sh
cargo new web-leptos
```

We'll also want to add this new project to our Cargo workspace, by editing the
root `Cargo.toml` file.

```toml
[workspace]
members = ["shared", "web-leptos"]
```

Now we can `cd` into the `web-leptos` directory and start fleshing out our
project. Let's add some dependencies to `shared/Cargo.toml`.

```toml
{{#include ../../../../examples/hello_world/web-leptos/Cargo.toml}}
```

```admonish tip
If using nightly Rust, you can enable the "nightly" feature for Leptos.
When you do this, the signals become functions that can be called directly.

However in our examples we are using the stable channel and so have to use
the `get()` and `update()` functions explicitly.
```

We'll also need a file called `index.html`, to serve our app.

```html
{{#include ../../../../examples/hello_world/web-leptos/index.html}}
```

## Create some UI

### Hello World counter example

```admonish example
There is slightly more complex [example](https://github.com/redbadger/crux/tree/master/examples/counter) of a Leptos app in the Crux repository.

However, the simplest example is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world), which has `shared` and `shared_types` libraries that will work with the following example code.
```

Edit `src/main.rs` to look like this:

```rust,noplayground
{{#include ../../../../examples/hello_world/web-leptos/src/main.rs}}
```

## Build and serve our app

The easiest way to compile the app to WebAssembly and serve it in our web page
is to use [`trunk`](https://trunkrs.dev/), which we can install with
[Homebrew](https://brew.sh/) (`brew install trunk`) or Cargo
(`cargo install trunk`).

We can build our app, serve it and open it in our browser, in one simple step.

```sh
trunk serve --open
```

```admonish success
Your app should look like this:

<p align="center"><img alt="hello world app" src="./hello_world.webp"  width="300"></p>
```
