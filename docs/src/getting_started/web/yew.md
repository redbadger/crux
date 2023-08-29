# Web — Rust and Yew

These are the steps to set up and run a simple Rust Web app that calls into a
shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](../core.md).
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
{{#include ../../../../examples/simple_counter/web-yew/Cargo.toml}}
```

We'll also need a file called `index.html`, to serve our app.

```html
{{#include ../../../../examples/simple_counter/web-yew/index.html}}
```

## Create some UI

```admonish example
There are several, more advanced,
[examples](https://github.com/redbadger/crux/tree/master/examples) of Yew apps
in the Crux repository.

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
by sending it directly back to the Yew component. Note that we wrap the effect
in a Message enum because Yew components have a single associated type for
messages and we need that to include both the events that the UI raises (to send
to the core) and the effects that the core uses to request side effects from the
shell.

Also note that because both our core and our shell are written in Rust (and run
in the same memory space), we do not need to serialize and deserialize the data
that we pass between them. We can just pass the data directly.

```rust,noplayground
{{#include ../../../../examples/simple_counter/web-yew/src/core.rs}}
```

```admonish tip
That `match` statement, above, is where you would handle any other effects that
your core might ask for. For example, if your core needs to make an HTTP
request, you would handle that here. To see an example of this, take a look at
the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter/web-yew/src/core.rs)
in the Crux repository.
```

Edit `src/main.rs` to look like the following. The `update` function is
interesting here. We set up a `Callback` to receive messages from the core and
feed them back into Yew's event loop. Then we test to see if the incoming
message is an `Event` (raised by UI interaction) and if so we use it to update
the core, returning false to indicate that the re-render will happen later. In
this app, we can assume that any other message is a render `Effect` and so we
return true indicating to Yew that we _do_ want to re-render.

```rust,noplayground
{{#include ../../../../examples/simple_counter/web-yew/src/main.rs}}
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
