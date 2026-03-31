# Notes example

Collaborative note editor demonstrating real-time sync between browser tabs.

## What this example shows

- **Text editing** with [Automerge](https://automerge.org/) CRDTs — every edit
  is a conflict-free operation that can be merged with edits from other peers
- **Real-time sync** via the browser's
  [BroadcastChannel](https://developer.mozilla.org/en-US/docs/Web/API/BroadcastChannel)
  API — open two tabs and watch edits appear in both
- **Custom capabilities** — a `PubSub` capability for publishing and subscribing
  to changes, and `crux_kv` for persistent storage
- **Timed auto-save** with `crux_time` — edits are persisted after 1 second of
  inactivity

## How to try it

1. `cd web-nextjs`
2. `just doctor` to check you have the right tools
3. `just serve` to start the dev server
4. Open the app in two browser tabs
5. Type in one tab and watch the text appear in the other

## Architecture

The `shared` directory contains the Crux core with:

- Events for text editing: `Insert`, `Replace`, `MoveCursor`, `Select`,
  `Backspace`, `Delete`
- A `Note` backed by an Automerge document, supporting conflict-free merges
- A custom `PubSub` capability (`capabilities/pub_sub.rs`) — the shell
  implements this using `BroadcastChannel` to sync between tabs
- A debounced save timer — edits are persisted to `crux_kv` after 1 second of
  inactivity
- Tests covering editing, cursor management, save/load, and multi-peer sync

## Shells

- NextJS — `web-nextjs/`

> **Note:** This example currently only has a web shell because the sync
> mechanism relies on `BroadcastChannel`, which is a browser API. Native shells
> (iOS, Android) would need a different transport (e.g. WebSocket server) to
> demonstrate sync, and are not yet implemented.
