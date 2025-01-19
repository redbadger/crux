# Shell/Core bridge performance tester

This example benchmarks the message passing between the shell and the core.
All versions attempt to send as many messages across the bridge as they can,
and the core responds with `Render` to every one of them. This way we measure
how many full interaction-to-ui-update roundtrips we can do per second.

### Notes:

1. This example currently depends on the `pnpm` package manager when generating
   types for TypeScript. We are currently revisiting the type generation for
   foreign types and so this requirement will probably go, but for now, please
   [install `pnpm`](https://pnpm.io/installation).

## The core

There are 2 events — `Tick` and `NewPEriod` that operate on a
simple model struct with a `count` field. Tick increments the count, new period
stores the current count in a log and rests the count, this allows us to draw
a historical chart of the message throughput.
