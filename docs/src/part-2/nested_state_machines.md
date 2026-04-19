# Nested state machines

In the previous chapter we saw the top-level `Model` behave as a lifecycle state machine: each phase is its own variant, each transition is explicit, and the `Outcome` type is how a stage tells the parent what to do next. That pattern isn't reserved for the top level — it runs all the way down. Every screen inside `Active`, and every workflow inside those screens, is its own small state machine, composed the same way.

This chapter zooms in on that nesting: the `Outcome` protocol itself, a worked example of a small state machine, and how transitions from deep inside the hierarchy can bubble all the way back up to the lifecycle.

## The Outcome pattern

Three types do all the work. First, the result of any sub-state-machine step:

```rust
{{#include ../../../examples/weather/shared/src/model/outcome.rs:outcome}}
```

An `Outcome` is a `Status` — either `Continue(State)` (the machine keeps running with the updated state) or `Complete(Transition)` (the machine has exited, here's the value telling the parent what happens next) — paired with a `Command` describing any effects the update produced.

```rust
{{#include ../../../examples/weather/shared/src/model/outcome.rs:status}}
```

And the counterpart for starting a state machine up:

```rust
{{#include ../../../examples/weather/shared/src/model/outcome.rs:started}}
```

A `start()` returns a `Started<Self, Event>` — the initial state bundled with the commands that kick off the work. The `map_event` methods on both types lift a child's event variant into a parent's wider event type, which is how events are routed through the hierarchy without each layer needing to know about the others.

That's the whole protocol. Now let's see it in use.

## A worked example: local weather

The home screen shows two things: local weather and weather for saved favourites. The local-weather half is a small state machine in its own right:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:state}}
```

The states map directly to what the UI shows: we're checking permissions, location is disabled, we're fetching coordinates, we're fetching weather, we have weather, or the fetch failed. Each state is moved forward by an event:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:event}}
```

Starting the state machine kicks off the first effect — asking the shell whether location services are enabled:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:start}}
```

This is the `Started` pattern we first saw in chapter 3, now at a lower level. The `update` function walks through each event and returns an `Outcome`:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:update}}
```

Most branches return `Outcome::continuing` — the machine keeps running with the new state, and a new command is attached (fetch location, fetch weather, render the disabled panel). Only one path completes the machine: a 401 from the weather API, which returns `Outcome::complete` with the single transition this machine exposes:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/local.rs:transition}}
```

That `Unauthorized` transition is how `LocalWeather` tells its parent: "I'm done; the API key is no longer valid."

## Nesting: HomeScreen composes two sub-workflows

`HomeScreen` contains `LocalWeather` alongside a second workflow that fetches weather for each saved favourite. The home-screen events reflect that:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/mod.rs:event}}
```

`HomeEvent::Local(...)` and `HomeEvent::FavoritesWeather(...)` are how the parent carries events for each sub-workflow. Shell-sent events go through `HomeEvent::GoToFavorites`; the others are internal routing, which is why they're marked `#[serde(skip)]` and `#[facet(skip)]`.

Starting the home screen starts both sub-workflows in parallel:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/mod.rs:start}}
```

Each child `start()` returns a `Started<ChildState, ChildEvent>`; `map_event` lifts the child's event type (`LocalWeatherEvent`, `FavoriteWeatherEvent`) into the parent's `HomeEvent`. The two commands are combined with `Command::and` and returned as a single `Started<HomeScreen, HomeEvent>`.

Updating is symmetric — unwrap the parent event, delegate to the child's update, match on the resulting status:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/mod.rs:update}}
```

For each sub-workflow branch, a `Continue` re-packages the updated state back into a fresh `HomeScreen`, while a `Complete` gets mapped to a `HomeTransition`. That's where the 401 path becomes interesting.

## Propagating transitions upward

A `LocalWeatherTransition::Unauthorized` doesn't escape `HomeScreen` as-is. It's lifted to a `HomeTransition`:

```rust
{{#include ../../../examples/weather/shared/src/model/active/home/mod.rs:transition}}
```

`HomeTransition::ApiKeyRejected(Favorites)` carries the current favourites list along, because whatever comes next still needs them. The active-model update does the same lift: it maps `HomeTransition::ApiKeyRejected` to `ActiveTransition::Unauthorized`, still carrying the favourites. The top-level `update_active` then sees `Complete(Unauthorized)` — exactly the handler we wrote in chapter 3 — and swaps `Model::Active` for `Model::Onboard`.

That's the full round trip: a 401 from the weather API, three levels below the top of the model tree, propagates up through three transition types until it becomes a lifecycle change. Each level decides what to do with its child's transition — either pass it along (lifted into its own transition type) or handle it locally.

## Debouncing with VersionedInput

One more pattern comes up inside the favourites workflow. When the user types in the "add favourite" search box, we want to fetch geocoding results — but we don't want a response for "Londo" to replace a response for "London" that arrives moments later. The answer is a small helper:

```rust
{{#include ../../../examples/weather/shared/src/model/versioned_input.rs:versioned_input}}
```

Every keystroke bumps the version. When we fire the geocoding request, we capture the current version. When the response arrives, we check whether the captured version still matches — if not, a newer search has happened, so we discard this result.

This isn't a state machine on its own, but the discipline is the same: make invalid states impossible to represent. Without a version, a stale response and a fresh one are both just strings — indistinguishable. With it, every response carries the version it was fired against, so the ambiguity simply can't happen. `VersionedInput` is used inside the add-favourite workflow, which is itself a nested state machine under favourites management.

## Next: making it all happen

So far we've modelled the state machines and talked about the commands they return, but we've treated commands as a black box — just "the thing that makes effects happen." In the next chapter, we'll look at the `Command` type properly: how effects are expressed, how commands compose, and how the protocol between the core and the shell actually works.
