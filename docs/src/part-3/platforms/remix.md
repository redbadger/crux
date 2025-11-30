# Web — TypeScript and React (Remix)

These are the steps to set up and run a simple TypeScript Web app that calls
into a shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](../core.md).
```

```admonish info
There are many frameworks available for writing Web applications with JavaScript/TypeScript. We've chosen [React](https://reactjs.org/) with [Remix](https://remix.run/) for this walk-through. However, a similar setup would work for other frameworks.
```

## Create a Remix App

For this walk-through, we'll use the [`pnpm`](https://pnpm.io/) package manager
for no reason other than we like it the most! You can use `npm` exactly the same
way, though.

Let's create a simple Remix app for TypeScript, using `pnpx` (from `pnpm`). You
can give it a name and then probably accept the defaults.

```sh
pnpx create-remix@latest
```

## Compile our Rust shared library

When we build our app, we also want to compile the Rust core to WebAssembly so
that it can be referenced from our code.

To do this, we'll use
[`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/), which you can
install like this:

```sh
# with homebrew
brew install wasm-pack

# or directly
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Now that we have `wasm-pack` installed, we can build our `shared` library to
WebAssembly for the browser.

```sh
(cd shared && wasm-pack build --target web)
```

````admonish tip
  You might want to add a `wasm:build` script to your `package.json`
  file, and call it when you build your Remix project.

  ```json
  {
    "scripts": {
      "build": "pnpm run wasm:build && remix build",
      "dev": "pnpm run wasm:build && remix dev",
      "wasm:build": "cd ../shared && wasm-pack build --target web"
    }
  }
  ```
````

Add the `shared` library as a Wasm package to your `web-remix` project

```sh
cd web-remix
pnpm add ../shared/pkg
```

We want to tell the Remix server to bundle our `shared` Wasm package, so we need
to add a `serverDependenciesToBundle` key to the object exported in
`remix.config.js`:

```js
{{#include ../../../../examples/simple_counter/web-remix/remix.config.js}}
```

## Add the Shared Types

To generate the shared types for TypeScript, we can just run `cargo build` from
the root of our repository. You can check that they have been generated
correctly:

```sh
ls --tree shared_types/generated/typescript
shared_types/generated/typescript
├── bincode
│  ├── bincodeDeserializer.d.ts
│  ├── bincodeDeserializer.js
│  ├── bincodeDeserializer.ts
│  ├── bincodeSerializer.d.ts
│  ├── bincodeSerializer.js
│  ├── bincodeSerializer.ts
│  ├── mod.d.ts
│  ├── mod.js
│  └── mod.ts
├── node_modules
│  └── typescript -> .pnpm/typescript@4.8.4/node_modules/typescript
├── package.json
├── pnpm-lock.yaml
├── serde
│  ├── binaryDeserializer.d.ts
│  ├── binaryDeserializer.js
│  ├── binaryDeserializer.ts
│  ├── binarySerializer.d.ts
│  ├── binarySerializer.js
│  ├── binarySerializer.ts
│  ├── deserializer.d.ts
│  ├── deserializer.js
│  ├── deserializer.ts
│  ├── mod.d.ts
│  ├── mod.js
│  ├── mod.ts
│  ├── serializer.d.ts
│  ├── serializer.js
│  ├── serializer.ts
│  ├── types.d.ts
│  ├── types.js
│  └── types.ts
├── tsconfig.json
└── types
   ├── shared_types.d.ts
   ├── shared_types.js
   └── shared_types.ts
```

You can see that it also generates an `npm` package that we can add directly to
our project.

```sh
pnpm add ../shared_types/generated/typescript
```

## Load the Wasm binary when our Remix app starts

The `app/entry.client.tsx` file is where we can load our Wasm binary. We can
import the `shared` package and then call the `init` function to load the Wasm
binary.

```admonish
Note that we `import` the wasm binary as well — Remix will automatically bundle
it for us, giving it a cache-friendly hash-based name.
```

```ts
{{#include ../../../../examples/simple_counter/web-remix/app/entry.client.tsx}}
```

## Create some UI

```admonish example
We will use the [simple counter example](https://github.com/redbadger/crux/tree/master/examples/simple_counter), which has `shared` and `shared_types` libraries that will work with the following example code.
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
{{#include ../../../../examples/simple_counter/web-remix/app/core.ts}}
```

```admonish tip
That `switch` statement, above, is where you would handle any other effects that
your core might ask for. For example, if your core needs to make an HTTP
request, you would handle that here. To see an example of this, take a look at
the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter/web-remix/src/core.rs)
in the Crux repository.
```

#### Create a component to render the UI

Edit `app/routes/_index.tsx` to look like the following. Notice that we pass the
`setState` hook to the update function so that we can update the state in
response to a render effect from the core (as seen above).

```typescript
{{#include ../../../../examples/simple_counter/web-remix/app/routes/_index.tsx}}
```

Now all we need is some CSS.

To add a CSS stylesheet, we can add it to the `Links` export in the
`app/root.tsx` file.

```tsx
{{#include ../../../../examples/simple_counter/web-remix/app/root.tsx:links}}
```

## Build and serve our app

We can build our app, and serve it for the browser, in one simple step.

```sh
pnpm dev
```

```admonish success
Your app should look like this:

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
