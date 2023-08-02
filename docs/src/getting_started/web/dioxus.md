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
{{#include ../../../../examples/hello_world/web-dioxus/Cargo.toml}}
```

## Create some UI

### Hello World counter example

```admonish example
There is slightly more complex [example](https://github.com/redbadger/crux/tree/master/examples/counter) of a Dioxus app in the Crux repository.

However, the simplest example is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world), which has `shared` and `shared_types` libraries that will work with the following example code.
```

Edit `src/main.rs` to look like this:

```rust,noplayground
{{#include ../../../../examples/hello_world/web-dioxus/src/main.rs}}
```

We can add a title and a stylesheet by editing
`examples/hello_world/web-dioxus/Dioxus.toml`.

```toml
{{#include ../../../../examples/hello_world/web-dioxus/Dioxus.toml}}
```

## Build and serve our app

Now we can build our app and serve it in one simple step.

```sh
dx serve
```

```admonish success
Your app should look like this:

<p align="center"><img alt="hello world app" src="./hello_world.webp"  width="300"></p>
```
