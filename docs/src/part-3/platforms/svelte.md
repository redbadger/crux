# Web - TypeScript and Svelte (SvelteKit)

These are the steps to set up and run a simple TypeScript Web app that calls
into a shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo,
as described in [Shared core and types](../core.md).
```

```admonish info
There are many frameworks available for writing Web applications with JavaScript/TypeScript.
We've chosen [Svelte](https://svelte.dev/) with [SvelteKit](https://svelte.dev/docs/kit/introduction)
for this walk-through. However, a similar setup would work for other frameworks.
```

## Create a Svelte App

Let's create a new project (using `pnpm`) which we'll call `web-svelte`:

```sh
pnpx sv create web-svelte
cd web-svelte
pnpm install
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

Now that we have `wasm-pack` installed, we can test building our `shared` library
to WebAssembly for the browser.

```sh
pushd ../shared
wasm-pack build --target web
popd
```

Edit the `package.json` file and add the `wasm:build` script:

```json
  "scripts": {
    "wasm:build": "cd ../shared && wasm-pack build --target web",
    "dev": "pnpm run wasm:build && vite dev",
    "build": "pnpm run wasm:build && vite build",
    "preview": "pnpm run wasm:build && vite preview",
  },
```

Also make sure to add the `shared` and `shared_types` as local dependencies to
the `package.json`, as well as the bincode package for serialization:

```json
  "dependencies": {
    "shared": "file:../shared/pkg",
    "shared_types": "file:../shared_types/generated/typescript",
    "bincode": "file:../shared_types/generated/typescript/bincode",
  }
```

After our Wasm has been built, we'll need to load it into our app. There are
several ways to do this, but the `simple_counter` example uses the
[`vite-plugin-wasm-esm`](https://github.com/omnysecurity/vite-plugin-wasm-esm) plugin.

```sh
pnpm add -D vite-plugin-wasm-esm
```

Now we have the plugin installed, we can configure Vite to use it to load our Wasm module.

```typescript
// file: vite.config.ts
{{#include ../../../../examples/simple_counter/web-svelte/vite.config.ts}}
```

We are creating a Single Page Application (SPA) using Svelte, so we will
need an adapter to build static files.

```sh
pnpm add -D @sveltejs/adapter-static
```

And we'll need to add a layout that indicates we don't need Server-Side Rendering (SSR).

```typescript
// file: src/routes/+layout.js
{{#include ../../../../examples/simple_counter/web-svelte/src/routes/+layout.js}}
```

Finally, our Svelte configuration can be finished:

```typescript
// file: svelte.config.js
{{#include ../../../../examples/simple_counter/web-svelte/svelte.config.js}}
```

#### Create an app to render the UI

Let's add [`bulma`](https://bulma.io/) for styling:

```sh
pnpm add bulma
```

Create a root page for our app at `src/routes/+page.svelte`:

```typescript
{{#include ../../../../examples/simple_counter/web-svelte/src/routes/+page.svelte}}
```

#### Wrap the core to support capabilities

Let's add a file `src/lib/core.ts` which will wrap our core and handle the
capabilities that we are using.

```typescript
{{#include ../../../../examples/simple_counter/web-svelte/src/lib/core.ts}}
```

This code sends our (UI-generated) events to the core, and handles any effects
that the core asks for via the `update()` function. Notice that we are creating
a [store](https://svelte.dev/docs/svelte-store) to update and manage the view
model. Whenever `update()` gets called to send an event to the core, we are
fetching the updated view model via `view()` and are updating the value in the
store. Svelte components can import and use the store values.

Notice that we have to serialize and deserialize the data that we pass between
the core and the shell. This is because the core is running in a separate
WebAssembly instance, and so we can't just pass the data directly.

## Build and serve our app

We can build our app, and serve it for the browser, in one simple step.

```sh
pnpm dev --open
```

```admonish success
Your app should look like this:

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
