# App lifecycle

As we think about the weather app, there's an overall workflow it moves through:

1. **Uninitialized** — the default; the core exists, but the shell hasn't kicked things off yet.
2. **Initialising** — triggered by `Event::Start`: retrieve resources that may have been saved previously (the API key, saved favourites).
3. **Onboarding** — if there's no API key, ask the user for one.
4. **Active** — we have everything we need; the app is running normally.
5. **Failed** — something went wrong that we can't recover from.

These phases are mutually exclusive — the app is always in exactly one of them — which makes a Rust enum the natural fit. Each variant holds the state for its phase, and we can focus on one at a time with its own events and transitions.

## The shape of the lifecycle

```rust
{{#include ../../../examples/weather/shared/src/model/mod.rs:model}}
```

The events driving it are namespaced by stage:

```rust
{{#include ../../../examples/weather/shared/src/model/mod.rs:event}}
```

`Event::Start` is the only event that kicks the app out of `Uninitialized`; the rest carry sub-events for a specific stage. `Initializing` is marked `#[serde(skip)]` and `#[facet(skip)]` because those events are internal to the core — the shell never sends them.

## Kicking things off

The top-level `update` function is small — it just decides which handler to dispatch to:

```rust
{{#include ../../../examples/weather/shared/src/model/mod.rs:update}}
```

`Event::Start` builds the `Initializing` state by calling `InitializingModel::start()`, which returns the initial model and the commands to run. Everything else is routed to a stage-specific `update_*` method.

But the core doesn't run itself — the shell has to send that `Event::Start` to begin with. Here are the iOS and Android shells doing exactly that:

```swift
{{#include ../../../examples/weather/apple/WeatherApp/WeatherApp.swift:start}}
```

```kotlin
{{#include ../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/Core.kt:start}}
```

In both cases the shell constructs the core, wires up its dependencies, and then immediately sends `Event::Start` — nothing else happens until the shell makes that first call. That's the "core is driven" point from chapter 2 in practice: the core is just a library until the shell pokes it.

## The transition pattern

`Event::Start` does its own transition right in the top-level `update` — it constructs the `Initializing` model and assigns it directly. For the other events, the top-level `update` delegates to a stage-specific handler. Those handlers all share the same shape, and it's worth looking at once before we dive into initialising.

Here's `update_initializing`:

```rust
{{#include ../../../examples/weather/shared/src/model/mod.rs:lifecycle_transition}}
```

Three moves:

1. **Take ownership** of the current model with `std::mem::take`. Because `Model` derives `Default`, this leaves `self` temporarily as `Uninitialized` — we're about to replace it, so that's fine.
2. **Delegate to the stage-specific update**, which returns an `Outcome<State, Transition, Event>`. The `Outcome` pairs a `Status` — either `Continue(State)` to stay in this phase or `Complete(Transition)` to exit — with a `Command` that represents the effects of the update.
3. **Put a model back.** For `Continue`, wrap the updated state back into the current phase. For `Complete`, construct the next phase's model and swap to it.

This `mem::take` → delegate → reassign shape takes advantage of Rust's ownership model. The stage-specific update takes `self` by value, so the model moves in, transforms, and comes back through `Outcome` — no cloning, with the type system enforcing that we reconstruct a model to put back. The `Outcome` itself is the protocol that tells the top level which phase comes next. We'll apply it to initialising in the next section, and in chapter 4 we'll see it used at every level inside `Active` too.

## Initialising: two fetches in parallel

Constructing an `InitializingModel` isn't just about the state — we also need to fire off the two fetches. A `Default` impl would give us the state, but nothing would actually start running. So instead we have a `start()` method that returns both the initial model _and_ the commands to run alongside it, paired up as a `Started<Self, Event>`:

```rust
{{#include ../../../examples/weather/shared/src/model/initializing.rs:start}}
```

Two commands, kicked off in parallel with `Command::all`: one to fetch the API key, one to read the favourites list from the KV store. Each binds its response to a specific `InitializingEvent` variant. We'll see more of the `Started` pattern in the next chapter.

Meanwhile, the state we're waiting in:

```rust
{{#include ../../../examples/weather/shared/src/model/initializing.rs:model}}
```

When a response comes back, it flows through `update`:

```rust
{{#include ../../../examples/weather/shared/src/model/initializing.rs:update}}
```

Each branch stores the result, then calls `resolve()` to see whether we have enough to move on:

```rust
{{#include ../../../examples/weather/shared/src/model/initializing.rs:resolve}}
```

Three cases:

1. Both fetched, key present → `Complete` with a transition to `Active`.
2. Both fetched, key missing → `Complete` with a transition to `Onboard`.
3. One still in flight → `Continue` with the updated state, and ask for a render so the loading screen keeps showing.

Back in the top-level `update_initializing`, both `Complete` cases follow the same shape: call the destination stage's `start()`, swap to the new `Model` variant, and compose the commands. `OnboardModel::start` returns a render so the onboarding screen appears; `ActiveModel::start` wraps `HomeScreen::start` to kick off the weather and location fetches.

## Onboard, Active, and Failed

`Onboard` looks much like `Initializing`: its own model, its own events, its own update, and its own transitions. When the user enters an API key and it's stored successfully, it transitions to `Active`. If storage fails, it transitions to `Failed`.

`Active` is where most of the app lives — the home screen with local weather and favourites, and the favourites management screen. That's the subject of the next chapter.

`Failed` is a dead end. It just carries a message for the UI to show. There's no event that leaves it.

## Back to onboarding

Not every lifecycle transition goes forward. Two things can send `Active` back to `Onboard`: the weather API returning a 401 (the stored key is bad), or the user explicitly asking to reset their key. Either way, `Active` completes with a transition carrying the current favourites and an `OnboardReason` — the onboarding flow is the same one we saw on first run; the reason is only used to pick the right message for the UI.

## Next: the pattern underneath

Every stage in this lifecycle — `Initializing`, `Onboard`, `Active` — returned an `Outcome`. The top-level `update_*` methods all matched on `Status::Continue` vs `Status::Complete(...)`, put the model back where it belongs, and composed commands. That's not a coincidence. It's the pattern the whole app is built on, all the way down to the individual screen workflows. That's what the next chapter is about.
