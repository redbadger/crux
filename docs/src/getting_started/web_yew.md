# Web — Rust and Yew

These are the steps to set up and run a simple Rust Web app that calls into a
shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](./core.md).
```

```admonish info
There are many frameworks available for writing Web applications in Rust. We've chosen [Yew](https://yew.rs/) for this walk-through because it is arguably the most mature. However, a similar setup would work for any framework that compiles to WebAssembly.
```

## Create a Yew App

Our Yew app is just a new Rust project, which we can create with Cargo. For this
example we'll call it `web-yew`.

```sh
cargo new web-yew
```

We'll also want to add this new project to our Cargo workspace, by editing the
root `Cargo.toml` file.

```toml
[workspace]
members = ["shared", "web-yew"]
```

Now we can start fleshing out our project. Let's add some dependencies to
`web-yew/Cargo.toml`.

```toml
{{#include ../../../examples/hello_world/web-yew/Cargo.toml}}
```

We'll also need a file called `index.html`, to serve our app.

```html
{{#include ../../../examples/hello_world/web-yew/index.html}}
```

## Create some UI

### Hello World counter example

```admonish example
There are several [examples](https://github.com/redbadger/crux/tree/master/examples) of Yew apps in the Crux repository.

However, the simplest example is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world), which has `shared` and `shared_types` libraries that will work with the following example code.
```

Edit `src/main.rs` to look like this:

```rust,noplayground
{{#include ../../../examples/hello_world/web-yew/src/main.rs}}
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

<p align="center"><img alt="hello world app" src="./hello_world_yew.webp"  width="300"></p>
```
