# Migrating from Capabilities to Command

Early versions of Crux performed side-effects through `Capabilities`: the app
declared a `Capabilities` type, `update` took a `caps` parameter, and
capabilities called back into the app with events. The `Command` API replaces
all of that. `update` returns a `Command`, and the `App` trait has no
`Capabilities` type or `caps` parameter at all.

The change is a breaking one for any app still on the old API. There are three
parts to the migration:

1. Remove the `Capabilities` associated type and the `caps` parameter from `update`.
2. Declare the `Effect` associated type on your app.
3. Return a `Command` from `update`.

Here's what the end state looks like:

```rust,ignore
// Rust
impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Effect = Effect;

    fn update(
        &self,
        event: Event,
        model: &mut Model,
    ) -> crux_core::Command<Effect, Event> {
        crux_core::Command::done() // return a Command
    }
}
```

To begin with, you can return `Command::done()` (a no-op) from `update` and
migrate your effect handling to Commands, and to capability APIs that return
command builders, one event at a time. [Managed Effects](../part-2/effects.md)
explains how Commands work.
