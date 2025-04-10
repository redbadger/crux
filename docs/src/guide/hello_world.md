# Hello world

As the first step, we will build a simple application, starting with a classic
Hello World, adding some state, and finally a remote API call. We will focus on
the core, rely on tests to tell us things work, and return to the shell a little
later, so unfortunately there won't be much to see until then.

If you want to follow along, you should start by following the
[Shared core and types](../getting_started/core.md), guide to set up the
project.

## Creating an app

```admonish example
You can find the full code for this part of the guide [here](https://github.com/redbadger/crux/blob/master/examples/hello_world/shared/src/app.rs)
```

To start with, we need a `struct` to be the root of our app.

```rust,noplayground
#[derive(Default)]
pub struct Hello;
```

We need to implement `Default` so that Crux can construct the app for us.

To turn it into an app, we need to implement the `App` trait from the
`crux_core` crate.

```rust,noplayground
use crux_core::App;

impl App for Hello {}
```

If you're following along, the compiler is now screaming at you that you're
missing five associated types for the trait — `Event`, `Model`, `ViewModel`,
`Capabilities`, (which will be deprecated soon, and can be set to `()`) and `Effect`.

The `Effect` associated type is worth understanding further, but in order to do that we need to talk about what makes Crux different from most UI frameworks.

## Side-effects and capabilities

One of the key design choices in Crux is that the Core is free of side-effects
(besides its internal state). Your application can never _perform_ anything that
directly interacts with the environment around it - no network calls, no
reading/writing files, and (somewhat obviously) not even updating the screen.
Actually _doing_ all those things is the job of the Shell, the core can only
_ask_ for them to be done.

This makes the core portable between platforms, and, importantly, really easy to
test. It also separates the intent, the "functional" requirements, from the
implementation of the side-effects and the "non-functional" requirements (NFRs).
For example, your application knows it wants to store data in a SQL database,
but it doesn't need to know or care whether that database is local or remote.
That decision can even change as the application evolves, and be different on
each platform. If you want to understand this better before we carry on, you can
read a lot more about how side-effects work in Crux in the chapter on
[Managed Effects](./effects.md).

To _ask_ the Shell for side effects, it will need to know what side effects it
needs to handle, so we will need to declare them (as an enum). _Effects_ are
simply messages describing what should happen, and for more complex side-effects
(e.g. HTTP), they would be too unwieldy to create by hand, so to help us create
them, Crux provides _capabilities_ - reusable libraries which give us a nice API
for requesting side-effects. We'll look at them in a lot more detail later.

Let's start with the basics:

```rust,noplayground
use crux_core::{
    macros::effect,
    render::RenderOperation,
};

#[effect]
pub enum Effect {
    Render(RenderOperation),
}
```

As you can see, for now, we will use a single capability, `crux_core::render`, which declares an `Operation` named `RenderOperation`, is built into Crux and is available from the `crux_core` crate. It simply tells the shell to update the screen using the latest information.

That means the core can produce a single `Effect`. It will soon be more than one, so we'll wrap it in an enum to give ourselves space. We'll also annotate our `Effect` enum with the `crux_core::macros::effect` attribute, which produces a _real_ `Effect` enum (which is very similar), one for FFI across the boundary to the shell, and various trait implementations and test helpers.

We also need to link the effect to our app. We'll go into the detail of why that is in the [Managed Effects](effects.md) section, but the basic reason is that capabilities need to be able to send the outcomes of their work back into the app.

You probably also noticed the `Event` type, which defines messages that can be sent back to the app. The same type is also used by the Shell to forward any user interactions to the Core, and in order to pass across the FFI boundary, it needs to be serializable. The resulting code will end up looking like this:

```rust,noplayground
use crux_core::{App, macros::effect, render::RenderOperation};
use serde::{Deserialize, Serialize};

#[effect]
pub enum Effect {
    Render(RenderOperation),
}

#[derive(Serialize, Deserialize)]
pub enum Event {
    None, // we can't instantiate an empty enum, so let's have a dummy variant for now
}

#[derive(Default)]
pub struct Hello;

impl App for Hello {}
```

Okay, that took a little bit of effort, but with this short detour out of the
way and foundations in place, we can finally create an app and start
implementing some behavior.

## Implementing the `App` trait

We now have almost all the building blocks to implement the `App` trait. We're
just missing two simple types. First, a `Model` to keep our app's state, it
makes sense to make that a struct. It needs to implement `Default`, which gives
us an opportunity to set up any initial state the app might need. Second, we
need a `ViewModel`, which is a representation of what the user should see on
screen. It might be tempting to represent the state and the view with the same
type, but in more complicated cases it will be too constraining, and probably
non-obvious what data are for internal bookkeeping and what should end up on
screen, so Crux separates the concepts. Nothing stops you using the same type
for both `Model` and `ViewModel` if your app is simple enough.

We'll start with a few simple types for events, model and view model.

Now we can finally implement the trait with its two methods, `update` and
`view`.

```rust,noplayground
{{#include ../../../examples/hello_world/shared/src/app.rs:app}}
```

The `update` function is the heart of the app. It responds to events by
(optionally) updating the state and requesting some effects by using the
capability's APIs.

All our `update` function does is ignore all its arguments and ask the Shell to
render the screen. It's a hello world after all.

The `view` function returns the representation of what we want the Shell to show
on screen. And true to form, it returns an instance of the `ViewModel` struct
containing `Hello World!`.

That's a working hello world done, lets try it. As we said at the beginning, for
now we'll do it from tests. It may sound like a concession, but in fact, this is
the intended way for apps to be developed with Crux - from inside out, with unit
tests, focusing on behavior first and presentation later, roughly corresponding
to doing the user experience first, then the visual design.

Here's our test:

```rust,noplayground
{{#include ../../../examples/hello_world/shared/src/app.rs:test}}
```

It is a fairly underwhelming test, but it should pass (check with `cargo test`).
The test uses a testing helper from `crux_core::testing` that lets us easily
interact with the app, inspect the effects it requests and its state, without
having to set up the machinery every time. It's not exactly complicated, but
it's a fair amount of boiler plate code.

## Counting up and down

```admonish example
You can find the full code for this part of the guide
[here](https://github.com/redbadger/crux/blob/master/examples/simple_counter/shared/src/app.rs)
```

Let's make things more interesting and add some behaviour. We'll teach the app
to count up and down. First, we'll need a model, which represents the state. We
could just make our model a number, but we'll go with a struct instead, so that
we can easily add more state later.

```rust,noplayground
#[derive(Default)]
pub struct Model {
    count: isize,
}
```

We need `Default` implemented to define the initial state. For now we derive it,
as our state is quite simple. We also update the app to show the current count:

```rust,noplayground
impl App for Hello {
// ...

    type Model = Model;

// ...

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        ViewModel {
            count: format!("Count is: {}", model.count),
        }
    }
}
```

We'll also need a simple `ViewModel` struct to hold the data that the Shell will
render.

```rust,noplayground
#[derive(Serialize, Deserialize)]
pub struct ViewModel {
    count: String,
}
```

Great. All that's left is adding the behaviour. That's where `Event` comes in:

```rust,noplayground
#[derive(Serialize, Deserialize)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
}
```

The event type covers all the possible events the app can respond to. "Will that
not get massive really quickly??" I hear you ask. Don't worry about that, there
is [a nice way to make this scale](./composing.md) and get reuse as well. Let's
carry on. We need to actually handle those messages.

```rust,noplayground
{{#include ../../../examples/simple_counter/shared/src/app.rs:impl_app}}
// ...
```

Pretty straightforward, we just do what we're told, update the state, and then
tell the Shell to render. Lets update the tests to check everything works as
expected.

```rust,noplayground
{{#include ../../../examples/simple_counter/shared/src/app.rs:test}}
```

Hopefully those all pass. We are now sure that when we build an actual UI for
this, it will _work_, and we'll be able to focus on making it looking
delightful.

In more complicated cases, it might be helpful to inspect the `model` directly.
It's up to you to make the call of which one is more appropriate, in some sense
it's the difference between black-box and white-box testing, so you should
probably be doing both to get the confidence you need that your app is working.

## Remote API

Before we dive into the thinking behind the architecture, let's add one more
feature - a remote API call - to get a better feel for how side-effects and
capabilities work.

```admonish example
You can find the full code for this part of the guide [here](https://github.com/redbadger/crux/blob/master/examples/counter/shared/src/app.rs)
```

We'll add a simple integration with a counter API we've prepared at
<https://crux-counter.fly.dev>. All it does is count up and down like our local
counter. It supports three requests

- `GET /` returns the current count
- `POST /inc` increments the counter
- `POST /dec` decrements the counter

All three API calls return the state of the counter in JSON, which looks
something like this

```json
{
  "value": 34,
  "updated_at": 1673265904973
}
```

We can represent that with a struct, and we'll need to update the model as well.
We can use Serde for the serialization (deserializing `updated_at` from
timestamp milliseconds to an option of `DateTime` using `chrono`).

We'll also update the count optimistically by keeping track of if/when the
server confirmed it (there are other ways to model these semantics, but let's
keep it straightforward for now).

```rust,noplayground
use chrono::{DateTime, Utc};
use chrono::serde::ts_milliseconds_option::deserialize as ts_milliseconds_option;

{{#include ../../../examples/counter/shared/src/app.rs:model}}
```

We also need to update the `ViewModel` and the `view()` function to display the
new data.

```rust,noplayground
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ViewModel {
    pub text: String,
    pub confirmed: bool,
}

...

fn view(&self, model: &Self::Model) -> Self::ViewModel {
    let suffix = match model.count.updated_at {
        None => " (pending)".to_string(),
        Some(d) => format!(" ({d})"),
    };

    Self::ViewModel {
        text: model.count.value.to_string() + &suffix,
        confirmed: model.count.updated_at.is_some(),
    }
}
```

You can see that the view function caters to two states - the count has not yet
been confirmed (`updated_at` is `None`), and having the count confirmed by the
server.

In a real-world app, it's likely that this information would be captured in a
struct rather than converted to string inside the core, so that the UI can
decide how to present it. The date formatting, however, is an example of
something you may want to do consistently across all platforms and keep inside
the Core. When making these choices, think about whose decisions they are, and
do they need to be consistent across platforms or flexible. You will no doubt
get a number of those calls wrong, but that's ok, the type system is here to
help you refactor later and update the shells to work with the changes.

We now have everything in place to update the `update` function. Let's start
with thinking about the events. The API does not support resetting the counter,
so that variant goes, but we need a new one to kick off fetching the current
state of the counter. The Core itself can't autonomously start anything, it is
always driven by the Shell, either by the user via the UI, or as a result of a
side-effect.

That gives us the following update function, with some placeholders:

```rust,noplayground
fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &(), // will be deprecated, so prefix with underscore for now
    ) -> Command<Effect, Event> {
    match event {
        Event::Get => {
            // TODO "GET /"
        }
        Event::Set(_response) => {
            // TODO Get the data and update the model
            render::render()
        }
        Event::Increment => {
            // optimistic update
            model.count.value += 1;
            model.count.updated_at = None;
            render::render()

            // real update
            // TODO "POST /inc"
        }
        Event::Decrement => {
            // optimistic update
            model.count.value -= 1;
            model.count.updated_at = None;
            render::render()

            // real update
            // TODO "POST /dec"
        }
    }
}
```

To request the respective HTTP calls, we'll use
[`crux_http`](https://github.com/redbadger/crux/tree/master/crux_http) the
built-in HTTP client. Since this is the first capability we're using, some
things won't be immediately clear, but we should get there by the end of this
chapter.

The first thing to know is that the HTTP responses will be sent back to the
update function as an event. That's what the `Event::Set` is for. The `Event`
type looks as follows:

```rust,noplayground
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Event {
    // these variants are used by the Shell
    Get,
    Increment,
    Decrement,

    // this variant is private to the Core
    #[serde(skip)]
    Set(crux_http::Result<crux_http::Response<Count>>),
}
```

We decorate the `Set` variant with `#[serde(skip)]` for two reasons: one,
there's currently a technical limitation stopping us easily serializing
`crux_http::Response`, and two, there's no reason that variant should ever be
sent by the Shell across the FFI boundary, which is the reason for the need to
serialize in the first place — in a way, it is private to the Core.

Finally, let's get rid of those TODOs. We'll need to add a variant to the
`Effect` enum, which holds the data for Http requests and responses.
In the snippet, below, `HttpRequest` is an implementation (in `crux_http`) of the `Operation` trait, which links the request and response types together.

```rust,noplayground
#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
}
```

We can now implement those TODOs, so lets do it. We're using the latest `Command` API
and so the `update` function will return a `Command` that has been created by
the `crux_http` and `render` capabilities (rather than using the `caps` parameter
in the method signature):

```rust,noplayground
const API_URL: &str = "https://crux-counter.fly.dev";

//...

fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
        _caps: &(), // will be deprecated, so prefix with underscore for now
    ) -> Command<Effect, Event> {
        match event {
            Event::Get => Http::get(API_URL)
                .expect_json()
                .build()
                .then_send(Event::Set),
            Event::Set(Ok(mut response)) => {
                let count = response.take_body().unwrap();
                model.count = count;
                render::render()
            }
            Event::Set(Err(_)) => {
                panic!("Oh no something went wrong");
            }
            Event::Increment => {
                // optimistic update
                model.count = Count {
                    value: model.count.value + 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/inc").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }
            Event::Decrement => {
                // optimistic update
                model.count = Count {
                    value: model.count.value - 1,
                    updated_at: None,
                };

                let call_api = {
                    let base = Url::parse(API_URL).unwrap();
                    let url = base.join("/dec").unwrap();
                    Http::post(url).expect_json().build().then_send(Event::Set)
                };

                render().and(call_api)
            }
        }
    }

```

There's a few things of note. The first one is that the `.then_send` API at the end
of each chain of calls to `crux_http` expects a function that wraps its argument
(a `Result` of a HTTP response) in a variant of `Event`. Fortunately, enum tuple
variants create just such a function, and we can use it. The way to read the
call is "Send a get request, parse the response as JSON, which should be
deserialized as a `Count`, and then call me again with `Event::Set` carrying the
result". Interestingly, we didn't need to specifically mention the `Count` type,
as the type inference from the `Event::Set` variant is enough, making it really
easy to read.

The other thing of note is that the Commands don't block. They queue up
requests to send to the shell and execution continues immediately. The requests
will be sent in the order they were queued and the asynchronous execution is the
job of the shell.

You can find the the complete example, including the tests and shell implementations
[in the Crux repo](https://github.com/redbadger/crux/blob/master/examples/counter/).
It's interesting to take a closer look at the unit tests:

```admonish example
These tests are taken from the Counter example
[implementation](https://github.com/redbadger/crux/blob/master/examples/counter/shared/src/app.rs).
```

```rust,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:simple_tests}}
```

For bigger `Model` or `ViewModel` structs, it may be easier to assert their correctness with
[`insta`](https://crates.io/crates/insta). If you don't know it already,
check it out. The really cool thing is that if the test fails, it shows you a
diff of the actual and expected output, and if you're happy with the new output,
you can accept the change (or not) by running `cargo insta review` — it will
update the code for you to reflect the change. It's a really nice way to do
snapshot testing, especially for the model and view model.

You can see how easy it is to check that the app is requesting the right side
effects, with the right arguments, and even test a chain of interactions and
make sure the behavior is correct, all without mocking or stubbing anything or
worrying about `async` code.

In the next chapter, we can put the example into perspective and discuss the
architecture it follows, inspired by Elm.
