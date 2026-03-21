# Terminal — Rust and Ratatui

These are the steps to set up and run a Crux app as a terminal UI (TUI)
application using [Ratatui](https://ratatui.rs/). This is a great way to
build lightweight, keyboard-driven interfaces that share the same core
logic as your web and mobile apps.

```admonish
This walk-through assumes you have already added the `shared` library to your repo, as described in [Shared core and types](../../part-1/shell.md).
```

```admonish info
Because both the core and the shell are written in Rust and run in the same
process, there is no FFI boundary — the shell calls the core directly with
no serialization overhead.
```

## Create the project

Our TUI app is just a new Rust project, which we can create with Cargo.

```sh
cargo new tui
```

Add it to your Cargo workspace by editing the root `Cargo.toml`:

```toml
[workspace]
members = ["shared", "tui"]
```

Add the dependencies to `tui/Cargo.toml`:

```toml
{{#include ../../../../examples/counter/tui/Cargo.toml}}
```

We depend on `shared` (our Crux core), `ratatui` (the TUI framework), and
`crossterm` (for terminal input handling).

## The shell

The entire TUI shell lives in a single `main.rs`. Let's walk through the key
parts.

```rust,noplayground
{{#include ../../../../examples/counter/tui/src/main.rs}}
```

### How it works

The TUI shell follows the same pattern as any Crux shell, but with a
terminal render loop instead of a UI framework:

1. **Event loop** — Ratatui runs a loop that draws the UI and then waits for
   keyboard input. Each keypress is mapped to an app `Event` (e.g. pressing
   `+` sends `Event::Increment`).

2. **Dispatching events** — The `dispatch` method sends events to the core via
   `core.process_event()` and processes the resulting effects. For this simple
   example, the only effect is `Render`, which is a no-op in the TUI — the
   shell re-renders on every loop iteration anyway.

3. **Rendering the view** — On each frame, the shell calls `core.view()` to
   get the current `ViewModel` and renders it using Ratatui widgets. The
   counter value is displayed in a bordered box with a row of selectable
   buttons below it.

4. **No serialization** — Because both the core and the shell are Rust running
   in the same process, we call `Core::new()`, `core.process_event()`, and
   `core.view()` directly with native Rust types.

## Build and run

```sh
cargo run -p tui
```

```admonish success
Your app should look something like this in the terminal:

```text
┏━━━━━━━━━━━━━━ Simple Counter ━━━━━━━━━━━━━━┓
┃                                             ┃
┃       Rust Core, Rust Shell (Ratatui)       ┃
┃                                             ┃
┃          ┌───────────────────┐              ┃
┃          │         0         │              ┃
┃          └───────────────────┘              ┃
┃                                             ┃
┃    ┃ Increment ┃  │ Decrement │  │  Reset  │┃
┃                                             ┃
┗━━ Select <←→> Confirm <Enter> Quit <Q> ━━━━┛
```
```
