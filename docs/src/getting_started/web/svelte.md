# Web - TypeScript and Svelte (Parcel)

These are the steps to set up and run a simple TypeScript Web app that calls
into a shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](../core.md).
```

```admonish info
There are many frameworks available for writing Web applications with JavaScript/TypeScript. We've chosen [Svelte](https://svelte.dev/) with [Parcel](https://parceljs.org/) for this walk-through. However, a similar setup would work for other frameworks.
```

## Create a Svelte App

Let's create a new project which we'll call `web-svelte`:

```sh
mkdir web-svelte
cd web-svelte
mkdir src/
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

Create a `package.json` file and add the `wasm:build` script:

```json
"scripts": {
    "wasm:build": "cd ../shared && wasm-pack build --target web",
    "start": "npm run build && concurrently -k \"parcel serve src/index.html --port 8080 --hmr-port 1174\" ",
    "build": "pnpm run wasm:build && parcel build src/index.html",
    "dev": "pnpm run wasm:build && parcel build src/index.html"
  },
```

Also make sure to add the `shared` and `shared_types` as local dependencies to the `package.json`:

```json
  "dependencies": {
    // ...
    "shared": "file:../shared/pkg",
    "shared_types": "file:../shared_types/generated/typescript"
    // ...
  }
```

#### Create an app to render the UI

Create a `main.ts` file in `src/`:

```typescript
{{#include ../../../../examples/simple_counter/web-svelte/src/main.ts}}
```

This file is the main entry point which instantiates a new `App` object.
The `App` object is defined in the `App.svelte` file:

```js
{{#include ../../../../examples/simple_counter/web-svelte/src/App.svelte}}
```

This file implements the UI and the behaviour for various user actions.


In order to serve the Svelte app, create a `index.html` in `src/`:

```html
{{#include ../../../../examples/simple_counter/web-svelte/src/index.html}}
```

This file ensures that the main entry point gets called.

#### Wrap the core to support capabilities

Let's add a file `src/core.ts` which will wrap our core and handle the
capabilities that we are using.

```typescript
{{#include ../../../../examples/simple_counter/web-svelte/src/core.ts}}
```

This code sends our (UI-generated) events to the core, and handles any effects that the core asks
for via the `update()` function. Notice that we are creating a [store](https://svelte.dev/docs/svelte-store)
to update and manage the view model. Whenever `update()` gets called to send an event to the core, we are
fetching the updated view model via `view()` and are udpating the value in the store. Svelte components can
import and use the store values.

Notice that we have to serialize and deserialize the data that we pass between
the core and the shell. This is because the core is running in a separate
WebAssembly instance, and so we can't just pass the data directly.

## Build and serve our app

We can build our app, and serve it for the browser, in one simple step.

```sh
npm start
```

```admonish success
Your app should look like this:

<p align="center"><img alt="simple counter app" src="./simple_counter.webp"  width="300"></p>
```
