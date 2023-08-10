# Simple Counter example

Simple counter example, with tests.

### Notes:

1. This example currently depends on the `pnpm` package manager when generating
   types for TypeScript. We are currently revisiting the type generation for
   foreign types and so this requirement will probably go, but for now, please
   [install `pnpm`](https://pnpm.io/installation).

## Simple Counter

There are 3 events — `Increment`, `Decrement` and `Reset` that operate on a
simple model struct with a `count` field.

There are tests to demonstrate how sending these events to the core, performs
the selected operation on the model and then uses the `Render` capability to ask
the UI to re-render.
