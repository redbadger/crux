# Web — TypeScript and React (Next.js)

These are the steps to set up and run a simple
TypeScript Web app that calls into a shared core.

```admonish
This walk-through assumes you have already set up the
`shared` library and codegen as described in
[Shared core and types](../../shell.md).
```

```admonish info
There are many frameworks available for writing Web
applications with JavaScript/TypeScript. We've chosen
[React](https://reactjs.org/) with
[Next.js](https://nextjs.org/) for this walk-through
because it is simple and popular. However, a similar
setup would work for other frameworks.
```

## Create a Next.js App

For this walk-through, we'll use the
[`pnpm`](https://pnpm.io/) package manager for no
reason other than we like it the most!

Let's create a simple Next.js app for TypeScript,
using `pnpx` (from `pnpm`). You can probably accept
the defaults.

```sh
pnpx create-next-app@latest
```

## Compile our Rust shared library

When we build our app, we also want to compile the
Rust core to WebAssembly so that it can be referenced
from our code.

To do this, we'll use
[`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/),
which you can install like this:

```sh
# with homebrew
brew install wasm-pack

# or directly
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

Now that we have `wasm-pack` installed, we can build
our `shared` library to WebAssembly for the browser.

```sh
wasm-pack build \
    --target web \
    --out-dir ../web-nextjs/generated/pkg \
    ../shared \
    --features wasm_bindgen
```

## Generate the Shared Types

To generate the shared types for TypeScript, we use the
codegen CLI we [prepared earlier](../../shell.md):

```sh
cargo run --package shared --bin codegen \
    --features codegen,facet_typegen \
    -- --language typescript \
       --output-dir generated/types
```

Both the Wasm package and the generated types are
referenced as local dependencies in `package.json`:

```json
{
  "dependencies": {
    "shared": "file:generated/pkg",
    "shared_types": "file:generated/types"
  }
}
```

Install the dependencies:

```sh
pnpm install
```

## Create some UI

```admonish example
There are other, more advanced,
[examples](https://github.com/redbadger/crux/tree/master/examples)
of Next.js apps in the Crux repository.

However, we will use the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter),
which has a `shared` library that will work with the
following example code.
```

### Counter example

A simple app that increments, decrements and resets a
counter.

#### Wrap the core to handle effects

First, let's add some boilerplate code to wrap our core
and handle the effects that it produces. For this
example, we only need to support the `Render` effect,
which triggers a render of the UI.

```admonish
This code that wraps the core only needs to be written
once — it only grows when we need to support additional
effects.
```

Edit `src/app/core.ts` to look like the following.
This code sends our (UI-generated) events to the core,
and handles any effects that the core asks for. In this
example, we aren't calling any HTTP APIs or handling
any side effects other than rendering the UI, so we
just handle this render effect by updating the
component's `view` hook with the core's ViewModel.

Notice that we have to serialize and deserialize the
data that we pass between the core and the shell. This
is because the core is running in a separate WebAssembly
instance, and so we can't just pass the data directly.

```typescript
{{#include ../../../../../examples/counter/web-nextjs/src/app/core.ts}}
```

```admonish tip
That `switch` statement, above, is where you would
handle any other effects that your core might ask for.
For example, if your core needs to make an HTTP
request, you would handle that here. To see an example
of this, take a look at the
[counter example](https://github.com/redbadger/crux/tree/master/examples/counter/web-nextjs/src/app/core.ts)
in the Crux repository.
```

#### Create a component to render the UI

Edit `src/app/page.tsx` to look like the following.
This code loads the WebAssembly core and sends it an
initial event. Notice that we pass the `setState` hook
to the update function so that we can update the state
in response to a render effect from the core.

```typescript
{{#include ../../../../../examples/counter/web-nextjs/src/app/page.tsx}}
```

Now all we need is some CSS. First add the `Bulma`
package, and then import it in `layout.tsx`.

```bash
pnpm add bulma
```

```typescript
{{#include ../../../../../examples/counter/web-nextjs/src/app/layout.tsx}}
```

## Build and serve our app

We can build our app, and serve it for the browser,
in one simple step.

```sh
pnpm dev
```

```admonish success
Your app should look like this:

<p align="center"><img alt="counter app" src="./counter.webp"  width="300"></p>
```
