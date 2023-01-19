# Capability APIs

In the previous chapter, we looked at the purpose of Capabilities and using them in Crux apps. In this one, we'll go through building our own. It will be a simple one, but real enough to show the key parts.

We'll extend the Counter example we've built in the [Hello World](hello_world.md) chapter and make it _worse_. Intentionally. We'll add a random delay before we actually update the counter, just to annoy the user (please don't do that in your real apps). It is a silly example, but it will allow us to demonstrate a few things:

- Random numbers, current time and delay are also side-effects
- To introduce a random delay, we will need to chain two effects behind a single capability call
- The capability can also offer specific time delay API and we can show how capabilities with multiple _operations_ work.

In fact, let's start with that.

## Basic delay capability

The first job of our capability will be to pause for a given number of milliseconds and then send an event to the app.

There's a number of types and traits we will need to implement to make the capability work with the rest of Crux, so let's quickly go over them berfore we start. We will need

- The capability itself, able to hold on to the context used to interact with Crux
- The payload type for the effect, holding the nuber of milliseconds requested
- Implementation of the `Capability` trait

Let's start with the payload:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DelayOperation {
    millis: usize
}
```

The request is just a named type holding onto a number. It will need to cross the FFI boundary, which is why it needs to be serializable, cloneable, etc.

We will need our request to implement the `Operation` trait, which links it with the type of the response we expect back. In our case we expect a response, but there is no data, so we'll use the unit type.

```rust
use crux_core::capability::Operation;

impl Operation for DelayOperation {
    type Output = ();
}
```

Now we can implement the capability:

```rust
use crux_core::capability::CapabilityContext;

struct Delay<Ev> {
    context: CapabilityContext<DelayOperation, Ev>,
}

impl<Ev> Delay<Ev> 
where
    Ev: 'static,
{
    pub new(context: CapabilityContext<DelayOperation, Ev>) -> Self {
        Self { context }
    }

    pub milliseconds(&self, millis: usize, event: Ev) {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            ctx.request_from_shell(DelayOperation { millis }).await

            ctx.update_app(event);
        });
    }
}
```

There's a fair bit going on. The capability is generic over an event type `Ev` and holds on to a `CapabilityContext`. The constructor will be called by Crux when starting an application that uses this capability.

The `milliseconds` method is our capability's public API. It takes the delay in milliseconds and the event to send back. In this case, we don't expect any payload to return, so we take the `Ev` type directly. We'll shortly see what an event with data looks like as well.

The implementation of the method has a little bit of boilerplate to enable us to use `async` code. First we clone the context to be able to use it in the async block. Then we use the context to spawn an `asyc move` code block in which we'll be able to use `async`/`await`. This bit of code will be the same in every part of your capability that needs to interact with the Shell.

You can see we use two APIs to orchestrate the interaction. First `request_from_shell` sends the delay operation we made earlier to the Shell. This call returns a future, which we can `.await`. Once done, we use the other API `update_app` to dispatch the event we were given. At the `.await`, the task will be suspended, Crux will pass the operation to the Shell wrapped in the `Effect` type we talked about in the last chapter and the Shell will use it's native APIs to wait for the given duration, and eventually respond. This will wake our task up again and we can continue working.

> 🚨 _SHARP EDGE WARNING_: There is one more thing we need to do, which will likely be reduced to a derive macro in future versions of Crux. We need to implement the `Capability` trait.

```rust
impl<Ef> Capability<Ef> for Delay<Ef> {
    type Operation = DelayOperation;
    type MappedSelf<MappedEv> = Delay<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        Delay::new(self.context.map_event(f))
    }
}
```

What on earth is that for, you ask? This allows you to derive an instance of the `Delay` capability from an existing one and adapt it to a different `Event` type. Yes, we know, don't read that sentence again. This will be useful to allow composing Crux apps from smaller Crux apps to automatically wrap the child events in the parent events.

We will cover this in depth in the chapter about [Composable applications](./composing.md).

## Random delays

To make the example more contrived, but also more educational, we'll add the random delay ability. This will

- Request a random number within given limits from the shell
- Then request the shell to delay by that number
- Then update the application, passing the number along, in case it is needed

First off, we need to add the new operation in. Here we have a choice, we can add a random delay operation, or we can add a random number generation operation and compose the two building blocks ourselves. We'll go for the second option because... well because this is an example.

Since we have multiple operations now, let's make our operation an enum

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    GetRandom(usize, usize),
    Delay(usize),
}
```

We now also need an output type:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOutput {
    Random(usize),
    TimeUp
}
```

And that changes the `Operation` trait implementation:

```rust
impl Operation for DelayOperation {
    type Output = DelayOutput;
}
```

The updated implementation looks like the following:

```rust
impl<Ev> Delay<Ev> 
where
    Ev: 'static,
{
    pub new(context: CapabilityContext<DelayOperation, Ev>) -> Self {
        Self { context }
    }

    pub milliseconds(&self, millis: usize, event: Ev) {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            ctx.request_from_shell(DelayOperation::Delay(millis)).await // Changed

            ctx.update_app(event);
        });
    }

    pub random<F>(&self, min: usize, max: usize, event: F)
    where F: Fn(usize) -> Ev 
    {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            let response = ctx.request_from_shell(DelayOperation::GetRandom(min, max)).await

            let millis = match response {
                Random(m) => m,
                _ => panic!("Expected a random number")
            }
            ctx.request_from_shell(DelayOperation::Delay(millis)).await

            ctx.update_app(event(millis));
        });
    }
}
```

In the new API, the event handling is a little different from the original. Because the event has a payload, we don't simply take an `Ev`, we need a function that returns `Ev`, if given the random number. Seems cumbersome but you'll see using it in the `update` function of our app is quite natural:

```rust
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

That is essentially it for the capabilities. You can check out the complete context API [in the docs](https://docs.rs/crux_core/latest/crux_core/capability/struct.CapabilityContext.html).
