# Notes example

Collaborative note editor with real-time sync. Demonstrates text editing with
[Automerge](https://automerge.org/) CRDTs, a custom PubSub capability for
broadcasting changes between peers, timed auto-save with `crux_time`, and
persistent storage with `crux_kv`.

## Architecture

The `shared` directory is a crate that implements the shared crux core. It contains:

- Events for text editing: `Insert`, `Replace`, `MoveCursor`, `Select`,
  `Backspace`, `Delete`
- A `Note` backed by an Automerge document, supporting conflict-free merges
- A custom `PubSub` capability (`capabilities/pub_sub.rs`) for publishing and
  subscribing to changes
- A debounced save timer — edits are persisted to `crux_kv` after 1 second of
  inactivity
- Tests covering editing, cursor management, save/load, and multi-peer sync

## Shells

- SwiftUI (iOS/macOS) — `apple/`
- Android/Kotlin — `Android/`
- Leptos — `web-leptos/`
- NextJS — `web-nextjs/`

## Running

1. Choose a shell you're interested in, i.e. `apple` or `Android`.
2. In the shell's directory, run `just doctor` to make sure you have the right
   tools installed
3. Run `just dev` to generate code and build that shell
4. For `apple` and `Android` shells, open the IDE. For others, run `just serve`
   in the shell directory.
