# Hello world

As the first step, we will build a simple application, starting with a classic Hello World, adding some state, and finally a remote API call. We will focus on the core, and return to the shell a little later, so unfortunately there won't be much to see until then.

If you want to follow along, you should start by following the [Shared core and types](../getting_started/core.md), guide to set up the project.

## Side-effects and capabilities

I know we said we'd build a simple application, but we need to start by talking about side-effects a little. One of the key design choices in Crux is that the Core is free of side-effects (besides its internal state). Your application can never _perform_ anything that directly interacts with the environment around it - no network calls, no reading/writing files, and (somewhat obviously) not even updating the screen. Actually _doing_ all those things is the job of the Shell, the core can only _ask_ for them to be done.

This makes the core portable between platforms, and, importantly, really easy to test. It also separates the intent, the "functional" requirements, from the implementation of the side-effects and the "non-functional" requirements (NFRs). For example, your application knows it wants to store data in a SQL database, but it doesn't need to know or care whether that database is local or remote. That decision can even change as the application evolves. You can read a lot more about how side-effects work in Crux in the chapter on [capabilities](./capabilities.md).

To _ask_ the Shell for side effects, we will need to describe the possible side-effects the Core will request. And to help us create these requests, we can use capabilities - reusable bits of functionality which give us a nice API for performing side-effects. We'll look at them in a lot more detail later.

```rust
use serde::{Serialize, Deserialize};
use crux_core::{render::Render};

#[derive(Serialize, Deserialize)]
enum Effect {
    Render
}

#[derive(Default)]
struct Capabilities {
    render: Render<Effect>
}

impl crux_core::Capabilities<Render<Effect>> {
    fn get(&self) -> &Render<Effect> {
        &self.render
    }
}
```

For now, we will need a single capability, `Render`, which is built into Crux and available from the `crux_core` crate. It simply tells the shell to update the screen using the latest information. That means the core also produces a single `Effect`, but it will soon be more than one, so we make Effect an enum to prepare for that. Effect also needs to be serializable, so it can be passed across the language boundary between Core and Shell.

> 🚨 **Sharp edge warning**
>
> The `crux_core::Capabilities` trait implementation will be derived with a macro in future versions of Crux.
> It provides a convenience API to access the capabilities from your app code which we'll see shortly.
> For now, it needs to be implemented by hand for all capabilities used by the app.

With this short detour out of the way, we can finally create and app and start implementing some behavior.

## Creating an app

To start with, we need a `struct` to be the root of our app.

```rust
use std::marker::PhantomData;

pub struct Hello<E, C> {
    _marker: PhantomData<fn() -> (E, C)>
};

impl Default for Hello {
    fn default() -> Self {
        Self {
            _marker: MarkerData
        }
    }
}
```

The struct is generic over two types parameters, `E` (for effect) and `C` (for capabilities). This supports composability and reusability of Crux apps (you can read more about that in the chapter on [Composing apps](./composing.md)). These are type parameters correspond to the types we defined in the previous section. We also need to implement `Default` so that Crux can construct the app for us.

To turn it into an app, we need to implement the `App` trait from the `crux_core` crate. And we'll need a few dependencies to make it work as well.

```rust
use serde::{Serialize};
use crux_core::{App, Capabilities, Command, render::Render};

impl<E, C> App<E, C> for Hello where 
    E: Clone + Serialize, 
    C: Capabilities<Render<E>> 
{
    type Event = ();
    type Model = ();
    type ViewModel = String;

    fn update(&self, _e: Event, _s: &mut Model, caps: &C) -> Vec<Command<Event>> {
        let render = caps::get<Render<_>>();

        vec![render.render()]
    }

    fn view(&self, model: &Model) -> ViewModel {
        "Hello World".to_string()
    }
}
```

There's a fair bit going on, so lets look at all of it one by one:

Like the `Hello` struct, the `impl` is generic over `E`, the effect and `C`, the capabilities. The effect type needs to support cloning and serialisation. The trait bound on `C` says that this app requires the capabilities provided to include the `Render` capability.

The trait has three associated types. The `Event` type describes the possible actions this app can perform. For now its's a unit type. The `Model` describes the internal state of the app, also a unit type for now, as there is no state to hold. The `ViewModel` describes the user interface to display.

The `update` function is the heart of the app. It responds to events by (optionally) updating the state and requesting some effects by returning `Command`s. The `Command` type bundles an effect with a message that the app should be sent when the effect is complete, but unfortunately for us, the `Render` effect does not return anything, which makes it a bad example to demonstrate the distinction between `Command` and `Effect`. We'll see a better one shortly.

All our `update` function does is ignore all its arguments and ask the Shell to render the screen.

Finally, the `view` function returns the representation of what we want the Shell to show on screen. To start with, it's a simple hello world.

That's a working hello world done, lets try it. For now we'll do it from tests.

```rust
#[cfg(test)]
mod tests {
    use crux_core::{Core, Command};
    use super::*;

    #[test]
    fn hello_says_hello_world() {
        let capabilities = Capabilities::default();
        let hello = Hello::default();
        let model = ();

        // Call 'update' and receive commands
        let commands = hello.update((), &mut model, &capabilities);
        
        let Command { effect: actual_effect, .. } = commands[0];
        let expected_effect = Effect::Render;

        // Check update asked us to `Render`
        assert_eq!(actual_effect, expected_effect);

        let actual_view = hello.view(&model);
        let expected_view = "Hello World".to_string();

        assert_eq!(actual_view, expected_view);
    }
}
```

It is a fairly underwhelming test, but it should pass (check with `cargo test`).

## Counting up and down

To make things more interesting, we'll add some behaviour. We'll teach the app to count up and down. First, we'll need a model, which represents the state. We could just use a number, but we'll use a struct instead, so that we can easily add more state later.

```rust
#[derive(Default)]
struct Model {
    count: isize,
}
```

We need `Default` implemented to define the initial state. For now we derive it, as our state is quite simple. We also update the app to use the model:

```rust
impl<E, C> App<E, C> for Hello where 
    E: Clone + Serialize, 
    C: Capabilities<Render<E>> 
{
// ...

    type Model = Model;

// ...

    fn view(&self, model: &Model) -> ViewModel {
        format!("Count is: {}", model.count)
    }
}
```

Great. All that's left is adding the behaviour. That's where `Event` comes in:

```rust
#[derive(Serialize, Deserialize)]
enum Event {
    Increment,
    Decrement,
    Reset,
}
```

The event type covers all the possible events we can respond to. "Will that not get massive really quickly??" I hear you ask. Don't worry about that, there is [a way to make this scale](./composing.md). Let's carry on. We need to actually handle those messages.

```rust
impl<E, C> App<E, C> for Hello where 
    E: Clone + Serialize, 
    C: Capabilities<Render<E>> 
{
    type Event = Event;
    type Model = Model;
    type ViewModel = String;

    fn update(&self, event: Event, state: &mut Model, caps: &C) -> Vec<Command<Event>> {
        let render = caps::get<Render<_>>();
        
        match event {
            Event::Increment => model.count += 1,
            Event::Decrement => model.count -= 1,
            Event::Reset => model.count = 0,
        };

        vec![render.render()]
    }
// ...
```

Pretty straightforward, we just do what we're told, update the state, and then tell the Shell to render. Lets update the tests to check everything works as expected.

```rust
#[cfg(test)]
mod test {
    use crux_core::{Core, Command};
    use super::*;

    #[test]
    fn renders() {
        let capabilities = Capabilities::default();
        let model = Model::default();
        let hello = Hello::default();

        let commands = hello.update(Event::Reset, &mut model, &capabilities);
        
        let Command { effect: actual_effect, .. } = commands[0];
        let expected_effect = Effect::Render;
    }

    #[test]
    fn shows_initial_count() {
        let capabilities = Capabilities::default();
        let model = Model::default();
        let hello = Hello::default();
 
        let actual_view = hello.view(&model)
        let expected_view = "Count is: 0";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let capabilities = Capabilities::default();
        let model = Model::default();
        let hello = Hello::default();
 
        hello.update(Event::Increment, &mut model, &capabilities);

        let actual_view = hello.view()
        let expected_view = "Count is: 1";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn decrements_count() {
        let capabilities = Capabilities::default();
        let model = Model::default();
        let hello = Hello::default();
 
        hello.update(Event::Decrement, &mut model, &capabilities);

        let actual_view = hello.view(&model)
        let expected_view = "Count is: -1";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn resets_count() {
        let capabilities = Capabilities::default();
        let model = Model::default();
        let hello = Hello::default();
 
        hello.update(Event::Increment, &mut model, &capabilities);
        hello.update(Event::Reset, &mut model, &capabilities);

        let actual_view = hello.view(&model)
        let expected_view = "Count is: 0";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn counts_up_and_down() {
        let capabilities = Capabilities::default();
        let model = Model::default();
        let hello = Hello::default();
 
        hello.update(Event::Increment, &mut model, &capabilities);
        hello.update(Event::Reset, &mut model, &capabilities);
        hello.update(Event::Decrement, &mut model, &capabilities);
        hello.update(Event::Increment, &mut model, &capabilities);
        hello.update(Event::Increment, &mut model, &capabilities);

        let actual_view = hello.view(&model)
        let expected_view = "Count is: 1";

        assert_eq!(actual_view, expected_view);
    }
}
```

Hopefully those all pass. We are now sure that when we build an actual UI for this, it will _work_, and we'll be able to focus on making it looking delightful.

## Remote API

Before we dive into the thinking behind the architecture, let's add one more feature - a remote API call - to get a better feel for how side-effects work.

**TO DO**