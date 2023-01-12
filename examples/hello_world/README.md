# Hello World example

Simple "hello world" and counter examples, with tests.

> Note: this is only a shared lib (and does not include any native shells).

## 1. Hello World

There is a single `None` event (enums with no variants cannot be instantiated).

There is a test to check that the `Render` capability asks the UI to re-render, and that the view returns the "Hello World!" string.

## 2. Counter

There are 3 events — `Increment`, `Decrement` and `Reset` that operate on a simple model struct with a `count` field.

There are tests to demonstrate how sending these events to the core, performs the selected operation on the model and then uses the `Render` capability to ask the UI to re-render.
