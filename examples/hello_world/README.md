# Hello World example

Simple "hello world" and counter examples, with tests.

> Note: this is only a shared lib (and does not include any native shells).

### Notes:

1. This example currently depends on the `pnpm` package manager when generating types for TypeScript. We are currently revisiting the type generation for foreign types and so this requirement will probably go, but for now, please [install `pnpm`](https://pnpm.io/installation).

## 1. Hello World

There is a single `None` event (enums with no variants cannot be instantiated).

There is a test to check that the `Render` capability asks the UI to re-render, and that the view returns the "Hello World!" string.

## 2. Counter

There are 3 events — `Increment`, `Decrement` and `Reset` that operate on a simple model struct with a `count` field.

There are tests to demonstrate how sending these events to the core, performs the selected operation on the model and then uses the `Render` capability to ask the UI to re-render.
