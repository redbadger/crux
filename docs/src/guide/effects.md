# Managed Effects

The approach to side-effects Crux uses is sometimes called **managed**
side-effects. Your app's core is not allowed to perform side-effects directly.
Instead, whenever it wants to interact with the outside world, it needs to
request the interaction from the shell.

Think of your whole app as a robot, where the Core is the brain of the robot and
the Shell is the body of the robot. The brain instructs the body through
capabilities and the body passes information about the outside world back to it
with Events.

```admonish warning title="Capability API Migration"
The side-effect API is in the middle of a migration. For the original API to request effects, see the
[next chapter about Capabilities](./capabilities.md).

The migration will happen in three stages:

1. Now: enable the Command API
2. Next: add Command based APIs to published capabilities
3. Later: remove original capability support
```

From `crux_core` version 0.11 there is a new API for side-effects which more
closely matches the mental model for Crux apps, in which the `update` function
_returns_ effect requests. It is facilitated by a new type - `Command`. The
`Command` encapsulates the entire process orchestrating a potentially complex
flow of side-effects resulting from this one `update` call.

In this chapter we will explore how commands are created and used, before we
dive into capabilities in the next chapter.

## Migrating from previous versions of Crux

The change to `Command` is a breaking one for all Crux apps, but the fix is
quite minimal.

There are two parts to it:

1. declare the `Effect` type on your App
2. return `Command` from `update`

Here's an example:

```rust
impl crux_core::App for App {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;

    type Capabilities = Capabilities;
    type Effect = Effect; // 1. add the associated type

    fn update(
        &self,
        event: Event,
        model: &mut Model,
        caps: &Capabilities,
    ) -> crux_core::Command<Effect, Event> {
        crux_core::Command::done() // 2. return a Command
    }
}

```

In a typical app the `Effect` will be derived from `Capabilities`, so the added
line should just work.

To begin with, you can simply return a `Command::done()` from the `update`
function. `Command::done()` is a no-op effect.

## What is a Command

The Command is a recipe for a side-effects workflow which may perform several
effects and also send events back to the app.

![Core, updated and command](../command_overview.png)

Crux expects Command to be returned by the `update` function. Command can be
asked for the effects and events that have been emitted so far. Each effect
carries a request for an operation (e.g. a HTTP request), which can be inspected
and resolved with an operation output (e.g. a HTTP response). After effect
requests are resolved, the command may have further effect requests or events,
depending on the recipe it's executing.

This API can be used both in tests and in Rust based shells, and for some
advanced use-cases when composing applications.

## Capabilities

Capabilities are developer-friendly, ergonomic APIs to construct commands, from
very basic ones all the way to complex stateful orchestrations.

For now, the capability API support is limited to directly created commands, and
the `Render` capability: instead of `caps.render.render()` you can call
`crux_core::render::render()` and return the command it builds.

## Working with Commands

```admonish warning "Work in progress"
This documentation is work in progress and will be expanded as we complete the
`Command` migration.

For now, you can read the [original RFC](../rfcs/command.md) explaining the reasoning behind the change,
And the [API docs](https://docs.rs/crux_core/latest/crux_core/struct.Command.html)
```
