# Hello world

As the first step, we will build a simple application, starting with a classic Hello World, adding some state, and finally a remote API call. We will focus on the core, rely on tests to tell us things work, and return to the shell a little later, so unfortunately there won't be much to see until then.

If you want to follow along, you should start by following the [Shared core and types](../getting_started/core.md), guide to set up the project.

## Creating an app

To start with, we need a `struct` to be the root of our app.

```rust
#[derive(Default)]
pub struct Hello;
```

We need to implement `Default` so that Crux can construct the app for us.

To turn it into an app, we need to implement the `App` trait from the `crux_core` crate.

```rust
use crux_core::App;

#[derive(Default)]
pub struct Model;

impl App for Hello {
```

If you're following along, the compiler is now screaming at you that you're missing four associated types for the trait: `Event`, `Model`, `ViewModel` and `Capabilities`.

Capabilities is the more complicated of them, and to understand what it does, we need to talk about what makes Crux different from most UI frameworks.

## Side-effects and capabilities

One of the key design choices in Crux is that the Core is free of side-effects (besides its internal state). Your application can never _perform_ anything that directly interacts with the environment around it - no network calls, no reading/writing files, and (somewhat obviously) not even updating the screen. Actually _doing_ all those things is the job of the Shell, the core can only _ask_ for them to be done.

This makes the core portable between platforms, and, importantly, really easy to test. It also separates the intent, the "functional" requirements, from the implementation of the side-effects and the "non-functional" requirements (NFRs). For example, your application knows it wants to store data in a SQL database, but it doesn't need to know or care whether that database is local or remote. That decision can even change as the application evolves, and be different on each platform. If you want to understand this better before we carry on, you can read a lot more about how side-effects work in Crux in the chapter on [capabilities](./capabilities.md).

To _ask_ the Shell for side effects, it will need to know what side effects it needs to handle, so we will need to declare them (as an enum). _Effects_ are simply messages describing what should happen, and for more complex side-effects (e.g. HTTP), they would be too unwieldy to create by hand, so to help us create them, Crux provides _capabilities_ - reusable libraries which give us a nice API for requesting side-effects. We'll look at them in a lot more detail later.

Let's start with the basics:

```rust
use crux_core::render::Render;

pub struct Capabilities {
    render: Render<Event>,
}
```

As you can see, for now, we will use a single capability, `Render`, which is built into Crux and available from the `crux_core` crate. It simply tells the shell to update the screen using the latest information.

That means the core can produce a single `Effect`. It will soon be more than one, so we'll wrap it in an enum to give ourselves space. The `Effect` enum corresponds one to one to the `Capabilities` we're using, and rather than typing it (and its associated trait implementations) by hand and open ourselves to unnecessary mistakes, we can use the `Effect` derive macro from the `crux_macros` crate.

```rust
use crux_core::{render::Render};
use crux_macros::Effect;

#[derive(Effect)]
#[effect(app = "Hello")]
pub struct Capabilities {
    render: Render<Event>,
}
```

Other than the `derive` itself, we also need to link the effect to our app. We'll go into the detail of why that is in the [Capabilities](capabilities.md) section, but the basic reason is that capabilities need to be able to send the app the outcomes of their work.

You probably also noticed the `Event` type which capabilities are generic over, because they need to know the type which defines messages they can send back to the app. The same type is also used by the Shell to forward any user interactions to the Core, and in order to pass across the FFI boundary, it needs to be serializable. The resulting code will end up looking like this:

```rust
use crux_core::{render::Render, App};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

#[derive(Effect)]
#[effect(app = "Hello")]
pub struct Capabilities {
    render: Render<Event>,
}

#[derive(Serialize, Deserialize)]
pub enum Event {
    None, // we can't instantiate an empty enum, so let's have a dummy variant for now
}

#[derive(Default)]
pub struct Hello;

impl App for Hello { ... }
```

Okay, that took a little bit of effort, but with this short detour out of the way and foundations in place, we can finally create an app and start implementing some behavior.

## Implementing the `App` trait

We now have almost all the building blocks to implement the `App` trait. We're just missing two simple types. First, a `Model` to keep our app's state, it makes sense to make that a struct. It needs to implement `Default`, which gives us an opportunity to set up any initial state the app might need. Second, we need a `ViewModel`, which is a representation of what the user should see on screen. It might be tempting to represent the state and the view with the same type, but in more complicated cases it will be too constraining, and probably non-obvious what data are for internal bookkeeping and what should end up on screen, so Crux separates the concepts. Nothing stops you using the same type for both `Model` and `ViewModel` if your app is simple enough.

We'll start with a unit struct for model and a simple `String` for the view model.

Now we can finally implement the trait with its two methods, `update` and `view`.

```rust
use crux_core::{render::Render, App};
use crux_macros::Effect;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct Model;

#[derive(Serialize, Deserialize)]
pub enum Event;

impl App for Hello {
    type Event = Event;
    type Model = Model;
    type ViewModel = String;
    type Capabilities = Capabilities;

    fn update(&self, _event: Self::Event, _model: &mut Self::Model, caps: &Self::Capabilities) {
        caps.render.render();
    }

    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
        "Hello World".to_string()
    }
}
```

The `update` function is the heart of the app. It responds to events by (optionally) updating the state and requesting some effects by using the capabilitie's APIs.

All our `update` function does is ignore all its arguments and ask the Shell to render the screen. It's a hello world after all.

The `view` function returns the representation of what we want the Shell to show on screen. And true to form, it returns `"Hello World!"`.

That's a working hello world done, lets try it. As we said at the beginning, for now we'll do it from tests. It may sound like a concession, but in fact, this is the intended way for apps to be developed with Crux - from inside out, with unit tests, focusing on behavior first and presentation later, roughly corresponding to doing the user experience first, then the visual design.

Here's our test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crux_core::{render::RenderOperation, testing::AppTester};

    #[test]
    fn hello_says_hello_world() {
        let hello = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        // Call 'update' and request effects
        let update = hello.update(Event::None, &mut model);

        // Check update asked us to `Render`
        assert_eq!(update.effects[0], Effect::Render(RenderOperation));

        let actual_view = hello.view(&mut model);
        let expected_view = "Hello World".to_string();

        // Make sure the view matches our expectations
        assert_eq!(actual_view, expected_view);
    }
}
```

It is a fairly underwhelming test, but it should pass (check with `cargo test`). The test uses a testing helper from `crux_core::testing` that lets us easily interact with the app, inspect the effects it requests and its state, without having to set up the machinery every time. It's not exactly complicated, but it's a fair amount of boiler plate code.

## Counting up and down

Let's make things more interesting and add some behaviour. We'll teach the app to count up and down. First, we'll need a model, which represents the state. We could just make our model a number, but we'll go with a struct instead, so that we can easily add more state later.

```rust
#[derive(Default)]
struct Model {
    count: isize,
}
```

We need `Default` implemented to define the initial state. For now we derive it, as our state is quite simple. We also update the app to show the current count:

```rust
impl App for Hello {
// ...

    type Model = Model;

// ...

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
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

The event type covers all the possible events the app can respond to. "Will that not get massive really quickly??" I hear you ask. Don't worry about that, there is [a nice way to make this scale](./composing.md) and get reuse as well. Let's carry on. We need to actually handle those messages.

```rust

impl App for Hello {
    type Model = Model;
    type Event = Event;
    type ViewModel = String;
    type Capabilities = Capabilities;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match event {
            Event::Increment => model.count += 1,
            Event::Decrement => model.count -= 1,
            Event::Reset => model.count = 0,
        };

        caps.render.render();
    }

// ...
```

Pretty straightforward, we just do what we're told, update the state, and then tell the Shell to render. Lets update the tests to check everything works as expected.

```rust
#[cfg(test)]
mod test {
    use super::*;
    use crux_core::{render::RenderOperation, testing::AppTester};

    #[test]
    fn renders() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Reset, &mut model);

        assert_eq!(update.effects[0], Effect::Render(RenderOperation));
    }

    #[test]
    fn shows_initial_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 0";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn increments_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 1";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn decrements_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Decrement, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: -1";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn resets_count() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);
        app.update(Event::Reset, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 0";

        assert_eq!(actual_view, expected_view);
    }

    #[test]
    fn counts_up_and_down() {
        let app = AppTester::<Hello, _>::default();
        let mut model = Model::default();

        app.update(Event::Increment, &mut model);
        app.update(Event::Reset, &mut model);
        app.update(Event::Decrement, &mut model);
        app.update(Event::Increment, &mut model);
        app.update(Event::Increment, &mut model);

        let actual_view = app.view(&mut model);
        let expected_view = "Count is: 1";

        assert_eq!(actual_view, expected_view);
    }
}
```

Hopefully those all pass. We are now sure that when we build an actual UI for this, it will _work_, and we'll be able to focus on making it looking delightful.

In more complicated cases, it might be helful to inspect the `model` directly. It's up to you to make the call of which one is more appropriate, in some sense it's the difference between black-box and white-box testing, so you should probably be doing both to get the confidence you need that your app is working.

## Remote API

Before we dive into the thinking behind the architecture, let's add one more feature - a remote API call - to get a better feel for how side-effects work.

**TO DO**
