# Building capabilities

In the previous chapter, we looked at the purpose of Capabilities and using them
in Crux apps. In this one, we'll go through building our own. It will be a
simple one, but real enough to show the key parts.

We'll extend the Counter example we've built in the
[Hello World](hello_world.md) chapter and make it _worse_. Intentionally. We'll
add a random delay before we actually update the counter, just to annoy the user
(please don't do that in your real apps). It is a silly example, but it will
allow us to demonstrate a few things:

- Random numbers, current time and delay are also side-effects
- To introduce a random delay, we will need to chain two effects behind a single
  capability call
- The capability can also offer specific time delay API and we can show how
  capabilities with multiple _operations_ work.

In fact, let's start with that.

## Basic delay capability

The first job of our capability will be to pause for a given number of
milliseconds.

The main job we have is to define the protocol to express this:


```rust,noplayground
{{#include ../../../doctest_support/src/basic_delay.rs:operation}}
```

The operation is just a named type holding onto a number. It will need to cross
the FFI boundary, which is why it needs to be serializable, cloneable, etc.

We will need our request to implement the `Operation` trait, which links it with
the type of the response we expect back. In our case we expect a response, but
there is no data, so we'll use the unit type.

```rust,noplayground
{{#include ../../../doctest_support/src/basic_delay.rs:operation_impl}}
```

Now we can implement the capability:

```rust,noplayground
{{#include ../../../doctest_support/src/basic_delay.rs:functions}}
```

That's it - it's just a function. But it has an interesting type signature. First lets look at the body and then we can come back to it. In the body, we call `Command::request_from_shell` which is one of the shorthand constructors provided by `Command`. They pretty much mirror the `CommandContext` API we saw in the previous chapter, and return a builder.

In our case, we're making a request to the shell. We don't expect anything back from the shell, but we do expect the shell to _resolve_ the request when the delay has elapsed (otherwise we would use `notify_shell`).

Okay, with that, lets talk about the type signature. Our function is generic over an `Effect` and an `Event` type, which will be defined by the app using the capability. We do need them to be somewhat special though: we need both of them to be `Send`, because we may be sending them across thread boundaries (if the effect resolves on another thread), and we also need them not to be references (hence the `'static` lifetime). And finally, we need an Effect type which supports construction from a `Request` of our `DelayOperation`. This `From` is one of the things implemented for you by the `#[effect]` macro.

The returned builder is generic over three types - `Effect`, `Event` and _a_ future with no output. The actual third type is anonymous, it's the specific async block created inside `request_from_shell`. If our operation used a different type as `Output`, we'd expected to see it as the `Output` type of the `impl Future`.

## Random delays

To make the example more contrived, but also more educational, we'll add the random delay ability. This will

- Request a random number within given limits from the shell
- Then request the shell to delay by that number

First off, we need to add the new operation in. Here we have a choice, we can
add a random delay operation, or we can add a random number generation operation
and compose the two building blocks ourselves. We'll go for the second option
because... well because this is an example.

Since we have multiple operations now, let's make our operation an enum

```rust,noplayground
{{#include ../../../doctest_support/src/delay.rs:operation}}
```

We now also need an output type:

```rust,noplayground
{{#include ../../../doctest_support/src/delay.rs:output}}
```

And that changes the `Operation` trait implementation:

```rust,noplayground
{{#include ../../../doctest_support/src/delay.rs:operation_impl}}
```

The updated implementation looks like the following:

```rust,noplayground
{{#include ../../../doctest_support/src/delay.rs:functions}}
```

The code is not hugely more complicated - we use the `.then_request` chaining to chain the two builders, and we panic if the first request is resolved with an output different than the `::Random` variant, because it signals a developer error on the shell side.

Here is what our app looks like with delay added in:

```rust,noplayground
fn update(&self, event: Self::Event, model: &mut Self::Model, _caps: ()) {
    match event {
        //
        // ... Some events omitted
        //
        Event::Increment => {
            Delay::random(200, 800).then_send(Event::DoIncrement);
        }
        Event::DoIncrement(_millis) => {
            // optimistic update
            model.count.value += 1;
            model.confirmed = Some(false);

            render::render();

            // real update
            let base = Url::parse(API_URL).unwrap();
            let url = base.join("/inc").unwrap();

            Http::post(url.as_str()).expect_json().build().then_send(Event::Set);
        }
        Event::Decrement => {
            Delay::milliseconds(500).then_send(Event::DoIncrement);
        }
        Event::DoDecrement => {
            // optimistic update
            model.count.value -= 1;
            model.confirmed = Some(false);

            render::render();

            // real update
            let base = Url::parse(API_URL).unwrap();
            let url = base.join("/dec").unwrap();

            Http::post(url.as_str()).expect_json().build().then_send(Event::Set);
        }
    }
}
```

## Beyond basics

Capabilities can get quite complicated, but the basic principles stay the same - their APIs return command builders, and the difference is in what those builders do. More advanced capabilities might need to construct command builders directly and use the `async` API to do their work, even `spawning` tasks which run in a loop and communicate with other tasks, etc.

But for the basics, that is essentially it for the capabilities. You can check out the complete
command context API to see what can be done from inside command builders
[in the docs](https://docs.rs/crux_core/latest/crux_core/command/struct.CommandContext.html).

## Writing tests for capabilities

The easiest way to test capabilities is to create simple `Effect` and `Event` enums that represent the possible operations and outcomes of the capability's behavior. You can then use these enums to assert the expected behavior of the capability in your tests.

This is not dissimilar to how you would test an app, but you don't need the full apparatus that is provided by an implementation of the `App` trait. We can convert the builder that the capability function returns into a `Command` by following up with an `Event` send.

```rust, noplayground
{{#include ../../../doctest_support/src/delay.rs:tests}}
```

----

```admonish warning title="The rest of this page is deprecated!"
You can safely continue with the [next chapter](./testing.md).

The Capability APIs have gone through a big step change recently. Everything below this
point covers how you would previously build them.

This is only relevant to you if you are working on an existing Crux app, otherwise
feel free to skip to the next chapter, [`Testing`](./testing.md).
```

## Previous APIs for building Capabilities

We'll extend the Counter example we've built in the
[Hello World](hello_world.md) chapter and make it _worse_. Intentionally. We'll
add a random delay before we actually update the counter, just to annoy the user
(please don't do that in your real apps). It is a silly example, but it will
allow us to demonstrate a few things:

- Random numbers, current time and delay are also side-effects
- To introduce a random delay, we will need to chain two effects behind a single
  capability call
- The capability can also offer specific time delay API and we can show how
  capabilities with multiple _operations_ work.

In fact, let's start with that.

## Basic delay capability

The first job of our capability will be to pause for a given number of
milliseconds and then send an event to the app.

There's a number of types and traits we will need to implement to make the
capability work with the rest of Crux, so let's quickly go over them before we
start. We will need

- The capability itself, able to hold on to the context used to interact with
  Crux
- The payload type for the effect, holding the number of milliseconds requested
- Implementation of the `Capability` trait

Let's start with the payload:

```rust,noplayground
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DelayOperation {
    millis: usize
}
```

The request is just a named type holding onto a number. It will need to cross
the FFI boundary, which is why it needs to be serializable, cloneable, etc.

We will need our request to implement the `Operation` trait, which links it with
the type of the response we expect back. In our case we expect a response, but
there is no data, so we'll use the unit type.

```rust,noplayground
use crux_core::capability::Operation;

impl Operation for DelayOperation {
    type Output = ();
}
```

Now we can implement the capability:

```rust,noplayground
use crux_core::capability::CapabilityContext;
use crux_core::macros::Capability;

#[derive(Capability)]
struct Delay<Ev> {
    context: CapabilityContext<DelayOperation, Ev>,
}

impl<Ev> Delay<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<DelayOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn milliseconds(&self, millis: usize, event: Ev)
    where
        Ev: Send,
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            ctx.request_from_shell(DelayOperation { millis }).await;

            ctx.update_app(event);
        });
    }
}
```

There's a fair bit going on. The capability is generic over an event type `Ev`
and holds on to a `CapabilityContext`. The constructor will be called by Crux
when starting an application that uses this capability.

The `milliseconds` method is our capability's public API. It takes the delay in
milliseconds and the event to send back. In this case, we don't expect any
payload to return, so we take the `Ev` type directly. We'll shortly see what an
event with data looks like as well.

The implementation of the method has a little bit of boilerplate to enable us to
use `async` code. First we clone the context to be able to use it in the async
block. Then we use the context to spawn an `async move` code block in which
we'll be able to use `async`/`await`. This bit of code will be the same in every
part of your capability that needs to interact with the Shell.

You can see we use two APIs to orchestrate the interaction. First
`request_from_shell` sends the delay operation we made earlier to the Shell.
This call returns a future, which we can `.await`. Once done, we use the other
API `update_app` to dispatch the event we were given. At the `.await`, the task
will be suspended, Crux will pass the operation to the Shell wrapped in the
`Effect` type we talked about in the last chapter and the Shell will use its
native APIs to wait for the given duration, and eventually respond. This will
wake our task up again and we can continue working.

Finally, we need to implement the `Capability` trait. This is done for us by the
`#[derive(Capability)]` macro, but it is worth looking at the generated code:

```rust,noplayground
impl<Ev> Capability<Ev> for Delay<Ev> {
    type Operation = DelayOperation;
    type MappedSelf<MappedEv> = Delay<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Delay::new(self.context.map_event(f))
    }
}
```

What on earth is that for, you ask? This allows you to derive an instance of the
`Delay` capability from an existing one and adapt it to a different `Event`
type. Yes, we know, don't read that sentence again. This will be useful to allow
composing Crux apps from smaller Crux apps to automatically wrap the child
events in the parent events.

We will cover this in depth in the chapter about
[Composable applications](./composing.md).

## Random delays

To make the example more contrived, but also more educational, we'll add the
random delay ability. This will

- Request a random number within given limits from the shell
- Then request the shell to delay by that number
- Then update the application, passing the number along, in case it is needed

First off, we need to add the new operation in. Here we have a choice, we can
add a random delay operation, or we can add a random number generation operation
and compose the two building blocks ourselves. We'll go for the second option
because... well because this is an example.

Since we have multiple operations now, let's make our operation an enum

```rust,noplayground
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    GetRandom(usize, usize),
    Delay(usize),
}
```

We now also need an output type:

```rust,noplayground
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOutput {
    Random(usize),
    TimeUp
}
```

And that changes the `Operation` trait implementation:

```rust,noplayground
impl Operation for DelayOperation {
    type Output = DelayOutput;
}
```

The updated implementation looks like the following:

```rust,noplayground
impl<Ev> Delay<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<DelayOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn milliseconds(&self, millis: usize, event: Ev) {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            ctx.request_from_shell(DelayOperation::Delay(millis)).await; // Changed

            ctx.update_app(event);
        });
    }

    pub fn random<F>(&self, min: usize, max: usize, event: F)
    where F: Fn(usize) -> Ev
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let response = ctx.request_from_shell(DelayOperation::GetRandom(min, max)).await;

            let DelayOutput::Random(millis) = response else {
                panic!("Expected a random number")
            };
            ctx.request_from_shell(DelayOperation::Delay(millis)).await;

            ctx.update_app(event(millis));
        });
    }
}
```

In the new API, the event handling is a little different from the original.
Because the event has a payload, we don't simply take an `Ev`, we need a
function that returns `Ev`, if given the random number. Seems cumbersome but
you'll see using it in the `update` function of our app is quite natural:

```rust,noplayground
fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
    match event {
        //
        // ... Some events omitted
        //
        Event::Increment => {
            caps.delay.random(200, 800, Event::DoIncrement);
        }
        Event::DoIncrement(_millis) => {
            // optimistic update
            model.count.value += 1;
            model.confirmed = Some(false);
            caps.render.render();

            // real update
            let base = Url::parse(API_URL).unwrap();
            let url = base.join("/inc").unwrap();
            caps.http.post(url.as_str()).expect_json().send(Event::Set);
        }
        Event::Decrement => {
            caps.delay.milliseconds(500, Event::DoIncrement);
        }
        Event::DoDecrement => {
            // optimistic update
            model.count.value -= 1;
            model.confirmed = Some(false);
            caps.render.render();

            // real update
            let base = Url::parse(API_URL).unwrap();
            let url = base.join("/dec").unwrap();
            caps.http.post(url.as_str()).expect_json().send(Event::Set);
        }
    }
}
```

That is essentially it for the capabilities. You can check out the complete
context API
[in the docs](https://docs.rs/crux_core/latest/crux_core/capability/struct.CapabilityContext.html).
