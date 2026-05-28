# Web — TypeScript and React Router

These are the steps to set up and run a simple TypeScript Web app that calls
into a shared core.

```admonish
This walk-through assumes you have already set up the `shared` library and codegen as described in [Shared core and types](../../part-1/shell.md).
```

```admonish info
There are many frameworks available for writing Web applications with JavaScript/TypeScript. We've chosen [React](https://reactjs.org/) with [React Router](https://reactrouter.com/) for this walk-through. However, a similar setup would work for other frameworks.
```

## Create a React Router App

For this walk-through, we'll use the [`pnpm`](https://pnpm.io/) package manager
for no reason other than we like it the most! You can use `npm` exactly the same
way, though.

Let's create a simple React Router app for TypeScript with `pnpm`. You can give
it a name and then probably accept the defaults.

```sh
pnpm create react-router@latest
```

## Compile our Rust shared library

When we build our app, we also want to compile the Rust core to WebAssembly so
that it can be referenced from our code.

To do this, we'll use BoltFFI, which you can install like this:

```sh
cargo install boltffi_cli --version '=0.25.0' --locked
brew install binaryen # provides wasm-opt
```

The crate is `boltffi_cli`; it installs the `boltffi` binary used below.

Now that we have `boltffi` installed, we can build our `shared` library to
WebAssembly for the browser.

```sh
(cd shared && boltffi pack wasm)
```

````admonish tip
  You might want to add a `wasm:build` script to your `package.json` file, and
  call it when you build your React Router project.

  ```json
  {
    "scripts": {
      "build": "pnpm run wasm:build && react-router build",
      "dev": "pnpm run wasm:build && react-router dev",
      "wasm:build": "cd ../shared && boltffi pack wasm"
    }
  }
  ```
````

Add the `shared` library as a Wasm package to your `web-react-router` project:

```sh
cd web-react-router
pnpm add ./generated/pkg
```

We want Vite to bundle our `shared` Wasm package, so we register the wasm and
React Router plugins in `vite.config.ts`:

```ts
{{#include ../../../../examples/counter/web-react-router/vite.config.ts}}
```

## Add the Shared Types

To generate the shared types for TypeScript, run the `codegen` binary, telling
it which language to emit and where to put the output:

```sh
cargo run --package shared --bin codegen --features codegen,facet_typegen -- \
    --language typescript --output-dir generated/types
```

You can check that they have been generated correctly:

```sh
ls --tree generated/types
generated/types
├── app.ts          # your app's Event / Effect / ViewModel types
├── app.d.ts
├── bincode
│  ├── bincodeDeserializer.ts
│  ├── bincodeSerializer.ts
│  └── index.ts
├── serde
│  ├── binaryDeserializer.ts
│  ├── binarySerializer.ts
│  ├── deserializer.ts
│  ├── serializer.ts
│  ├── types.ts
│  └── index.ts
├── package.json
└── tsconfig.json
```

You can see that it also generates an `npm` package that we can add directly to
our project.

```sh
pnpm add ./generated/types
```

## Load the Wasm binary when our React Router app starts

The `app/entry.client.tsx` file is where we load our Wasm binary. We import the
`shared` package and wait for its `initialized` promise to resolve before
hydrating the app, so the WASM module is ready before any event reaches the
core.

```ts
{{#include ../../../../examples/counter/web-react-router/app/entry.client.tsx}}
```

## Create some UI

```admonish example
We will use the [simple counter example](https://github.com/redbadger/crux/tree/master/examples/counter), which has a `shared` library and generated TypeScript types that will work with the following example code.
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

Edit `app/core.ts` to look like the following. This code sends our
(UI-generated) events to the core, and handles any effects that the core asks
for. In this simple example, we aren't calling any HTTP APIs or handling any
side effects other than rendering the UI, so we just handle this render effect
by updating the component's `view` hook with the core's ViewModel.

Notice that we have to serialize and deserialize the data that we pass between
the core and the shell. This is because the core is running in a separate
WebAssembly instance, and so we can't just pass the data directly.

```typescript
{{#include ../../../../examples/counter/web-react-router/app/core.ts}}
```

```admonish tip
That `switch` statement, above, is where you would handle any other effects that
your core might ask for. For example, if your core needs to make an HTTP
request, you would handle that here. To see an example of this, take a look at
the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter/web-react-router/app/core.ts)
in the Crux repository.
```

#### Create a component to render the UI

Edit `app/routes/_index.tsx` to look like the following. Notice that we pass the
`setState` hook to the update function so that we can update the state in
response to a render effect from the core (as seen above).

```typescript
{{#include ../../../../examples/counter/web-react-router/app/routes/_index.tsx}}
```

Now all we need is some CSS.

To add a CSS stylesheet, we can add it to the `Links` export in the
`app/root.tsx` file.

```tsx
{{#include ../../../../examples/counter/web-react-router/app/root.tsx:links}}
```

## Build and serve our app

We can build our app, and serve it for the browser, in one simple step.

```sh
pnpm dev
```

```admonish success
Your app should look like this:

<p align="center"><img alt="simple counter app" src="../../part-1/shell/web/counter.webp"  width="300"></p>
```
