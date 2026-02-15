# Structuring larger apps

Now we have a better handle on what Crux apps are made of, lets have a think about how we might build our Weather app. It is certainly small enough to be built by just blindly following the simple counter example. There is only about 25 different events total, but you probably agree that some more structure would be good.

## Composition

Fortunately, all the key components of the architecture compose. We can have `Event` variants which carry other event types,
`Model` fields containing other models and `update` functions calling other module's update function. And looking at the main `app.rs` module of the Weather app, this is exactly what's going on:

Here's the Event

```rust
{{#include ../../../examples/weather/shared/src/app.rs:event}}
```

There are only three options - navigate somewhere, an event on the home screen, or an event in the Favourites section.

The `update` function reflects this too:

```rust
{{#include ../../../examples/weather/shared/src/app.rs:update}}

```

We'll look closer at the navigation in the next section, but the other two events simply forward to a different module's `update` function. In a special case we actually call two different updates from two different modules in response to the same event.
In this example, we pass down the whole model as is, but we could also just pass down a single field of it.

You can also see another kind of composition - a composition of commands. both `favorites::events:update` and `weather::events::update` return a `Command`, and the `Event::Home` branch uses `Command::all` to run those commands in parallel. You might be wondering what's going on with the `.map_event`. The `Command` returned by `favorites::events` can emit the `FavoritesEvent` type, but we need our commands to emit them wrapped in the `Event::Favorites` (and boxed, because they are a larger type), so that when they arrive back to this `update` function, they get recognized as favorites events and sent down the third branch of the `match`.

The main thing to remember about this is that the events always come in from the top, and they get routed by the layers to the right function which can process them (or they can be processed directly, if the parent module knows better and wants to do something special).

Model can compose in a similar way, but in our case it's more of a mix:

```rust
{{#include ../../../examples/weather/shared/src/app.rs:model}}
```

The `favorites` field is a type from the `favourites` module, but `weather_data` looks useful globally, so does `search_results` and the location related fields.

The most interesting of these is the `Workflow` type, which manages our navigation state - what page of the app are we currently on.

The main takeaway is that Crux is design such that whole apps can be composed - an existing type implementing `App` can be used, unchanged from a "parent" app, by

1. adding an event variant which carries the child's event
2. storing the child's model in the model
3. calling the child's `update` where appropriate
4. mapping the commands returned to the parent's event, and effect types (using `.map_event` and `.map_effect`)

That doesn't mean you should always subdivide apps in the same way, it is often a lot more convenient to share a model, or even a event type across two or more modules. Just know that should you need to reuse a whole Crux app later on, you can.

## Navigation

Typical apps involve some type of geography. The smaller the screen, the more moving between sections the user needs to do. But in principle, this is just more state, typically of the exclusive nature - the user can't be in two places at once. To
avoid thinking too much about screens or windows (what if we need to build a CLI or a VR version?), lets generalise this idea
in the concept of a `Workflow`. These are in no way a special type, we're simply modeling our domain in Rust.

In our Weather app, the `Workflow` is an enum:

```rust
{{#include ../../../examples/weather/shared/src/app.rs:workflow}}
```

In other words - the user can be either on the `Home` page, or in the `Favorites` section (which has some additional state), or they can be adding a favorite. No other options currently exist, and they can only be doing one of those things at once.

At this point, it might be helpful to look at how this is reflected in the view model:

```rust
{{#include ../../../examples/weather/shared/src/app.rs:view_model}}
```

It is _also_ an enum, because we're currently thinking about the app as separate workflows. If we had a two-panel kind of UX
with a list and detail, we might model this differently. It's worth spending some time thinking about this when building the app, and this is part of why we encourage building Crux apps from inside out.

The ViewModel's variants are a fair bit richer than the `Workflow` - while the workflow in the model is only concerned with where the user is, the ViewModel also carries the information they see. It is entirely enough for us to draw a user interface from.

To bring it home, lets look at the view function:

```rust
{{#include ../../../examples/weather/shared/src/app.rs:view}}
```

As you may have guessed, it maps the workflow to a view model, inserting some data from the model along the way.

That's enough to express the idea of navigation, and what workflow the user is meant to be in. How it specifically works on each platform is up to each Shell.
