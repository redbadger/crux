# Hello World example

A simple "hello world" counter example, with tests.

> Note: this is only a shared lib (and does not include any native shells).

There are 3 `Event`s — `Increment`, `Decrement` and `Reset` that operate on a simple model struct with a `count` field.

There are tests to demonstrate how sending these events to the core, performs the selected operation on the model and then uses the `Render` capability to ask the UI to re-render.
