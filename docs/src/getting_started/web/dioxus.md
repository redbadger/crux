# Web — Rust and Dioxus

These are the steps to set up and run a simple Rust Web app that calls into a
shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](../core.md).
```

```admonish info
There are many frameworks available for writing Web applications in Rust. We've chosen [Dioxus](https://dioxuslabs.com/) for this walk-through. However, a similar setup would work for other frameworks that compile to WebAssembly.
```

## Create a Dioxus App

````admonish tip
Dioxus has a CLI tool called `dx`, which can initialize, build and serve our app.

  ```sh
  cargo install dioxus-cli
  ```

Test that the executable is available.

  ```sh
  dx --help
  ```
````

Before we create a new app, let's add it to our Cargo workspace (so that the
`dx` tool won't complain), by editing the root `Cargo.toml` file.

For this example, we'll call the app `web-dioxus`.

```toml
[workspace]
members = ["shared", "web-dioxus"]
```

Now we can create a new Dioxus app. The tool asks for a project name, which
we'll provide as `web-dioxus`.

```sh
dx create

cd web-dioxus
```

Now we can start fleshing out our project. Let's add some dependencies to the
project's `Cargo.toml`.

```toml
{{#include ../../../../examples/simple_counter/web-dioxus/Cargo.toml}}
```

## Create some UI

```admonish example
There is slightly more advanced [example](https://github.com/redbadger/crux/tree/master/examples/counter) of a Dioxus app in the Crux repository.

However, we will use the [simple counter example](https://github.com/redbadger/crux/tree/master/examples/simple_counter), which has `shared` and `shared_types` libraries that will work with the following example code.
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
by updating the component's `view` hook with the core's ViewModel.

Also note that because both our core and our shell are written in Rust (and run
in the same memory space), we do not need to serialize and deserialize the data
that we pass between them. We can just pass the data directly.

```rust,noplayground
{{#include ../../../../examples/simple_counter/web-dioxus/src/core.rs}}
```

```admonish tip
That `match` statement, above, is where you would handle any other effects that
your core might ask for. For example, if your core needs to make an HTTP
request, you would handle that here. To see an example of this, take a look at
the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter/web-dioxus/src/core.rs)
in the Crux repository.
```

Edit `src/main.rs` to look like the following. This code sets up the Dioxus app,
and connects the core to the UI. Not only do we create a hook for the view state
but we also create a coroutine that plugs in the Dioxus "service" we defined
above to constantly send any events from the UI to the core.

```rust,noplayground
{{#include ../../../../examples/simple_counter/web-dioxus/src/main.rs}}
```

We can add a title and a stylesheet by editing
`examples/simple_counter/web-dioxus/Dioxus.toml`.

```toml
{{#include ../../../../examples/simple_counter/web-dioxus/Dioxus.toml}}
```

## Build and serve our app

Now we can build our app and serve it in one simple step.

```sh
dx serve
```

```admonish success
Your app should look like this:

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
