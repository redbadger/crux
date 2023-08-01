# Web — TypeScript and React (Next.js)

These are the steps to set up and run a simple TypeScript Web app that calls
into a shared core.

```admonish
This walk-through assumes you have already added the `shared` and `shared_types` libraries to your repo, as described in [Shared core and types](../core.md).
```

```admonish info
There are many frameworks available for writing Web applications with JavaScript/TypeScript. We've chosen [React](https://reactjs.org/) with [Next.js](https://nextjs.org/) for this walk-through because it is simple and popular. However, a similar setup would work for other frameworks.
```

## Create a Next.js App

For this walk-through, we'll use the [`pnpm`](https://pnpm.io/) package manager
for no reason other than we like it the most!

Let's create a simple Next.js app for TypeScript, using `pnpx` (from `pnpm`).
You can probably accept the defaults.

```sh
pnpx create-next-app@latest
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
  file, and call it when you build your nextjs project.

  ```json
  {
    "scripts": {
      "build": "pnpm run wasm:build && next build",
      "dev": "pnpm run wasm:build && next dev",
      "wasm:build": "cd ../shared && wasm-pack build --target web"
    }
  }
  ```
````

Add the `shared` library as a Wasm package to your `web-nextjs` project

```sh
cd web-nextjs
pnpm add ../shared/pkg
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

## Create some UI

### Hello World counter example

```admonish example
There are several [examples](https://github.com/redbadger/crux/tree/master/examples) of Next.js apps in the Crux repository.

However, the simplest example is the [Hello World counter example](https://github.com/redbadger/crux/tree/master/examples/hello_world), which has `shared` and `shared_types` libraries that will work with the following example code.
```

Edit `web-nextjs/pages/index.tsx` to look like this:

```typescript
{{#include ../../../../examples/hello_world/web-nextjs/src/app/page.tsx}}
```

Now all we need is some CSS. First add the `Bulma` package, and then import it
in `layout.tsx`.

```bash
pnpm add bulma
```

```typescript
{{#include ../../../../examples/hello_world/web-nextjs/src/app/layout.tsx}}
```

## Build and serve our app

We can build our app, and serve it for the browser, in one simple step.

```sh
pnpm dev
```

```admonish success
Your app should look like this:

<p align="center"><img alt="hello world app" src="./hello_world.webp"  width="300"></p>
```
