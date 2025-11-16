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
{{#include ../../../../examples/simple_counter/web-leptos/Cargo.toml}}
```

```admonish tip
If using nightly Rust, you can enable the "nightly" feature for Leptos.
When you do this, the signals become functions that can be called directly.

However in our examples we are using the stable channel and so have to use
the `get()` and `update()` functions explicitly.
```

We'll also need a file called `index.html`, to serve our app.

```html
{{#include ../../../../examples/simple_counter/web-leptos/index.html}}
```

## Create some UI

```admonish example
There is slightly more advanced
[example](https://github.com/redbadger/crux/tree/master/examples/counter) of a
Leptos app in the Crux repository.

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

Edit `src/core.rs` to look like the following. This code sends our
(UI-generated) events to the core, and handles any effects that the core asks
for. In this simple example, we aren't calling any HTTP APIs or handling any
side effects other than rendering the UI, so we just handle this render effect
by sending the new ViewModel to the relevant Leptos signal.

Also note that because both our core and our shell are written in Rust (and run
in the same memory space), we do not need to serialize and deserialize the data
that we pass between them. We can just pass the data directly.

```rust,noplayground
{{#include ../../../../examples/simple_counter/web-leptos/src/core.rs}}
```

```admonish tip
That `match` statement, above, is where you would handle any other effects that
your core might ask for. For example, if your core needs to make an HTTP
request, you would handle that here. To see an example of this, take a look at
the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter/web-leptos/src/core.rs)
in the Crux repository.
```

Edit `src/main.rs` to look like the following. This code creates two signals
— one to update the view (which starts off with the core's current view), and
the other to capture events from the UI (which starts of by sending the reset
event). We also create an effect that sends these events into the core whenever
they are raised.

```rust,noplayground
{{#include ../../../../examples/simple_counter/web-leptos/src/main.rs}}
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

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
