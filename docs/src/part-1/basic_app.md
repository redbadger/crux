# A very basic app

The basic app we'll build as an example to demonstrate the interaction between
the Shell and the Core and the state management will be the well known and loved
counter app. A simple counter we can increment, decrement and reset.

## Code of the app

```admonish example
You can find the full code for this part of the guide [here](https://github.com/redbadger/crux/blob/master/examples/simple_counter/shared/src/app.rs)
```

In the last chapter, we started with the main type

```rust,noplayground
#[derive(Default)]
pub struct Counter;
```

We need to implement `Default` so that Crux can construct the app for us.

To turn it into a Crux app, we need to implement the `App` trait from the
`crux_core` crate.

```rust,noplayground
use crux_core::App;

impl App for Counter {

}
```

If you're following along, the compiler is now screaming at you that you're
missing four associated types for the trait — `Event`, `Model`, `ViewModel`,
and `Effect`.

Let's add them and talk about them one by one.

## Event

Event defines all the possible events the app can respond to. It is essentially
the Core's public API.

In our case it will look as follows:

```rust,noplayground
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
}
```

Those are the three things we can do with the counter. None of them need any additional
information, so this simple `enum` will do. It is serializable, because it will
eventually be crossing the FFI boundary. We will get to that soon.

## Model

Model holds our application's internal state. You can probably guess what this will look like:

```rust
#[derive(Default)]
pub struct Model {
    count: isize,
}
```

It is a simple counter after all. Model stays in the core, so it doesn't need to serialize. 

You can derive (or implement) `Default` and have Crux create an instance of your app and your model for you, or you can explicitly create a core with specified App and Model instances (this may be useful if you need to set up some initial state).

## ViewModel

ViewModel represents the user interface at any one point in time. This is our indirection between
the internal state and the UI on screen. In the case of the counter, this is pretty
academic, there is no practical reason for making them different, but for the sake of the example,
let's add some formatting in the mix and make it a string.

```rust,noplayground
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ViewModel {
    pub count: String,
}
```

The difference between `Model` and `ViewModel` will get a lot more pronounced once we introduce
some navigation into the mix in Part II.

## Effect

For now, the counter has no side effects. Except it wants to update the user interface, and
that is also a side effect. We'll go with this:

```rust
use crux_core::macros::effect;
use crux_core::render::RenderOperation;

#[effect(typegen)]
#[derive(Debug)]
pub enum Effect {
    Render(RenderOperation),
}
```

We're saying "the only side effect of our behaviour is rendering the user interface".

The `Effect` type is worth understanding further, but in order to do that we need to
talk about what makes Crux different from most UI frameworks.

## Managed side-effects

One of the key design choices in Crux is that the Core is free of side-effects
(besides its internal state). Your application can never _perform_ anything that
directly interacts with the environment around it - no network calls, no
reading/writing files, not even updating the screen.
Actually _doing_ all those things is the job of the Shell, the core can only
_ask_ for them to be done.

This makes the core portable between platforms, and, importantly, very easy to
test. It also separates the intent – the "functional" requirements – from the
implementation of the side-effects and the "non-functional" requirements (NFRs).

For example, your application knows it wants to store data in a SQL database,
but it doesn't need to know or care whether that database is local or remote.
That decision can even change as the application evolves, and be different on
each platform. We won't go into the detail at this point, because we don't need
the full extent of side effects just yet. If you want to know more now, you can jump ahead
to the chapter on [Managed Effects](../part-2/effects.md), but it's probably a bit much
at this point. Up to you.

All you need to know for now is that for us to _ask_ the Shell for side effects,
it will need to know what side effects it needs to handle, so we will need to
list the possible kinds of effects (as an enum). _Effects_ are simply messages
describing what should happen. In our case the only option is asking for a UI update
(or, more precisely, telling the shell a new view model is available).

That's enough about effects for now, we will spend a lot more time with them later on.

## Implementing the `App` trait

We now have all the building blocks to implement the `App` trait. Here is
where we end up (straight from the actual [example code](https://github.com/redbadger/crux/blob/master/examples/simple_counter/shared/src/app.rs)):

```rust,noplayground
{{#include ../../../examples/simple_counter/shared/src/app.rs:impl_app}}
```

The `update` function is the heart of the app, it manages the state transitions
of the app. It responds to events by (optionally) updating the state. You
may have noticed the strange return type: `Command<Effect, Event>`.

This is the request for some side-effects. We seem to be accumulating terminology,
so let's do a quick recap:

- **Effect** - a request for a type of side-effect (e.g. a HTTP request)
- **Operation** - carried by the Effect, specifies the data for the effect (e.g. the URL, method, headers, body...)
- **Command** - a bundle of effect requests which execute together, sequentially, in parallel or in
  a more complex coordination

```admonish question title="Why so much layering?"
In real apps, we typically use a few kinds of effects over and over,
and so it's necessary to allow reuse. That's what the `Effect` enum does, it
bundles together effects of the same type, defined by the same module or crate (We
call those modules Capabilities, but lets not worry about those yet).

The other thing
that happens in real apps is mixing different kinds of effects in workflows, chaining
them, running them concurrently, even racing them. That's what commands allow you to do.
```

Our `update` function looks at the `event` it got, updates the `model.count`, and
since the count has changed, the UI needs to update, so it calls `render()`. The
`render()` call returns a `Command`, which `update` just passes on to the caller.

The `view` function's job is to return the representation of what we want the Shell to show
on screen. It's up to the Shell to call it when ready. Our view does a bit of string
formatting and wraps it in a `ViewModel`.

That's a working counter done. It's obviously really basic, but it's enough for us [to test
it](./testing.md).
