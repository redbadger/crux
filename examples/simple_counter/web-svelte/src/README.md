# Simple Counter Example with Svelte

This simple counter example is implemented as a [Svelte](https://svelte.dev/) app with [Parcel](https://parceljs.org/). 

## Getting Started

From your terminal run:

```sh
npm install .
npm start
```

Open http://localhost:8080

## Notes

There are 3 events — `Increment`, `Decrement` and `Reset` that operate on a
simple model struct with a `count` field.

The view model is managed via a [Svelte store](https://svelte.dev/docs/svelte-store) that gets updated automatically whenever
the `update()` function in `core.ts` is called to send an event to the core.
