# RFC: New side effect API - Command

This is a proposed implementation of a new API for creating (requesting)
side-effects in crux apps. It is quite a significant part of the Crux API
surface, so we'd really appreciate feedback on the direction this is taking.

## Why?

Why a new effect API, you may ask. Was there anything wrong with the original
one? Not really. Not critically wrong anyway. One could achieve all the
necessary work with it just fine, it enables quite complex effect orchestration
with async Rust and ultimately enables Crux cores to stay fully pure and
therefore be portable and very cheaply testable. This new proposed API is an
evolution of the original, building on it heavily.

However.

There's a number of paper cuts, obscured intentions, limitations, and misaligned
incentives which come with the original API:

### The original API is oddly imperative, but non-blocking.

A typical call to a capability looks roughly like:
`caps.some_capability.do_a_thing(inputs, Event::ThingDone)`. This call doesn't
block, but also doesn't return any value. The effect request is magically
registered, but is not represented as a value - it's gone. Fell through the
floor and into the Crux runtime.

Other than being a bit odd, this has two consequences. One is that it is
impossible to combine or modify capability calls in any other way than executing
them concurrently. This includes any type of transformation, like a declarative
retry strategy or a timeout and, in particular, cancellation.

The more subtle consequence (and my favourite soap box) is that it encourages a
style of architecture inside Crux apps, which Crux itself is working quite hard
to discourage overall.

The temptation is to build custom types and functions, pass capability instances
to them and call them deep down in the call stacks. This obscures the intended
mental model in which the `update` call mutates state and _results in_ some
side-effects - it returns them back to the shell to execute. Except in the
actual type signature of `update` it does not.

The intent of Crux is to very strictly separate code updating state from code
interacting with the outside world with side-effects. Borrowing ChatGPT's
analogy - separate the brain of the robot from its body. The instructions for
movement of the body should be an overall result of the brain's decision making,
not sprinkled around it.

The practical consequence of the sprinkling is that code using capabilities is
difficult to test fully without the `AppTester` for the same reason it is
difficult to test any other code doing I/O of any sort. In our case it's
practically impossible to test code using capabilities without the `AppTester`,
because creating instances of capabilities is not easy.

The original API doesn't make it obvious that the intent is to avoid this mixing
and so people using it tend to try to mix it and find themselves in various
levels of trouble.

**In summary**: Effects should be a return value from the `update` function, so
that all code generating side effects makes it very explicit, making it part of
its return type. That should in turn encourage more segregation of stateful but
pure logic from effects.

### Access to `async` code is limited to capabilities and orchestration of effects is limited to `async` code

This, like most things, started with good intention - people find async code to
be "doing Rust on hard mode", and so for the most part Crux is designed to avoid
async until you genuinely need it. Unfortunately it turned out to mean that you
need async (or loads of extra events) as soon as any level of effect chaining is
required.

For example - fetching an API auth token, followed by three HTTP API calls,
followed by writing each response to disk (or local storage) is a sequence of
purely side-effect operations. The recipe is known up front, and at no point are
any decisions using the model being made.

With the original API the choices were either to introduce intermediate events
between the steps of the effect state machine OR to implement the work in async
rust in a capability, which gets to spawn async tasks. The limitation there was
that the tasks can't easily call other capabilities.

With the
[introduction of the Compose capability](https://github.com/redbadger/crux/pull/179)
that last limitation was removed, allowing composition of effects across
capabilities, even within the `update` function body, so long as the
capabilities offer an async API. The result was calls to `caps.compose.spawn`
ending up _all over_ and leading to creation of a new _kind_ of type - a
capability orchestrator, for example an API client, which is built from a few
capabilities (lets say HTTP, KV and a custom authentication) _and_ Compose. This
kind of type is basically untestable on its own.

**In summary**: It should be possible to do simple orchestration of effects
without async code and gradually move into async code when its expressivity
becomes more convenient.

### Testing code with side-effects requires the AppTester

As covered above, the code producing side effects requires a special runtime in
order to run fully and be tested, and so any code using capabilities
automatically makes testing impossible without the AppTester. And of course apps
themselves are only testable with the AppTester harness.

**In summary**: It should be possible for tests to call the update function and
inspect the side effects like any other return value.

### Capabilities have various annoying limitations

- Capabilities are "tethered" to the crux core, allowing them to submit effects
  and events and spawn tasks, but it means instances of capabilities have to be
  created by Crux and injected into the `update` function.
- The first point means that capability authors don't have any control over
  their capability's main type, and therefore it's impossible for capabilities
  to have any internal state. For capabilities managing ongoing effects, like
  WebSocket connections for example, this is limiting. It can be worked around
  by the capability creating and returning values representing such ongoing
  connections, which can communicate with a separate task spawned by the
  capability over an async channel. The task read from the channel in a loop,
  and therefore can have local state. It works, but it is far from obvious.
- They have to offer two sets of APIs: one for event callback use, one for async
  use. This largely just adds boilerplate, but there is a clear pattern where
  the event callback calls simply use their async twins. That can be done by
  Crux and the boilerplate removed.
- They don't compose cleanly. With the exception of `Compose`, capabilities are
  expected to emit their own assigned variant of `Effect`, preordained at the
  time the capability instance is created. This doesn't specifically stop one
  capability from asking another to emit an effect, but it is impossible to
  modify the second capability's request in any way - block specific one, retry
  it, combine it with a timeout, all the way up to completely virtualising it:
  resolving the effect using other data, like an in-memory cache, instead of
  passing it to the shell.

**In summary**: Capabilities should not be special and should simply be code
which encapsulates

- the definition of the core/shell communication protocol for the particular
  type of effect
- creation of the requests and any additional orchestration needed for this
  communication

### App composition is not very flexible

This is really just another instance of the limitations of capabilities and the
imperative effect API. Any transformations involved in making an app a "child"
of another app has to be done up front. Specifically, this involves mapping or
wrapping effects and events of the child app onto the effect and event type of
the parent, typically with a `From` implementation which can't make contextual
decisions.

**In summary**: Instead, this mapping should be possible after the fact, and be
case specific. It should be possible for apps to partially virtualise effects of
the child apps, re-route or broadcast the events emitted by one child app to
other children, etc.

## How does this improve the situation?

So, with all (or at least most of) the limitations exposed, how is this proposed
API better?

### Enter `Command`

As others before us, such as [Elm](https://guide.elm-lang.org/effects/) and
[TCA](https://pointfreeco.github.io/swift-composable-architecture/main/tutorials/composablearchitecture/01-02-addingsideeffects#Performing-a-network-request),
we end up with a solution where `update` _returns_ effects. Specifically, it is
expected to returns an effect orchestration which is the result of the update
call as an instance of a new type called `Command`.

In a way, `Command` is the current effect executor in a box, with some extras.
Command is a lot like
[`FuturesUnordered`](https://docs.rs/futures/latest/futures/stream/futures_unordered/struct.FuturesUnordered.html) -
it holds one or more futures which it runs in the order they get woken up until
they all complete.

On top of this, Command provides to the futures a "context" - a type with an API
which allows the futures within the command to submit effects and events. This
is essentially identical to the current `CapabilityContext`. The difference is
that the effects and events get collected in the Command and can be inspected,
modified, forwarded or ignored.

In a general shape, Command is a stream of Effect or Events created as an
orchestration of futures. It also implements `Stream`, which means commands can
wrap other commands and use them for any kind of orchestration.

### Orchestration with or without async

Since Commands are values, we can work with them after they are created. It's
pretty simple to take several commands and join them into one which runs the
originals concurrently:

```rust
let command = Command::all([command_a, command_b, command_c]);
```

Commands provide basic constructors for the primitives:

- A command that does nothing (a no-op): `Command::done()`
- A command that emits an event: `Command::event(Event::NextEvent))`
- A command that sends a notification to the shell:
  `Command::notify_shell(my_notification)`
- A command that sends a request to the shell:
  `Command::request_from_shell(my_request).then_send(Event::Response)`
- A command that sends a stream request to the shell:
  `Command::stream_from_shell(my_request).then_send(Event::StreamEvent)`

Notice that the latter two use chaining to register the event handler. This is
because the other useful orchestration ability is chaining - creating a command
with a result of a previous command. This requires a form of the builder
pattern, since commands themselves are streams, not futures, and doing a simple
`.then` would require a fair bit of boilerplate.

Instead, to create a request followed by another request you can use the builder
pattern as follows:

```rust
let command = Command::request_from_shell(a_request)
    .then_request(|response| Command::request_from_shell(make_another_request_from(response)))
    .then_send(Event::Done);
```

This works just the same with streams or combinations of requests and streams.

`.then_*` and `Command::all` are nice, but on occasion, you will need the full
power of async. The equivalent of the above with async works like this:

```rust
let command = Command::new(|ctx| async {
    let response = ctx.request_from_shell(a_request).await;
    let second_response = ctx.request_from_shell(make_another_request_from(response)).await;

    ctx.send_event(Event::Done(second_response))
})
```

Alternatively, you can create the futures from command builders:

```rust
let command = Command::new(|ctx| async {
    let response = Command::request_from_shell(a_request)
        .into_future(ctx).await;
    let second_response = Command::request_from_shell(make_another_request_from(response))
        .into_future(ctx).await;

    ctx.send_event(Event::Done(second_response))
})
```

You might be wondering why that's useful, and the answer is that it allows
capabilities to return the result of `Command::request_from_shell` for simple
shell interactions and not worry about whether they are being used in a sync or
async context. It would be ideal if the command builders themselves could
implement `Future` or `Stream`, but unfortunately, to be useful to us, the
futures need access to the context which will only be created once the `Command`
itself is created.

### Testing without AppTester

Commands can be tested by inspecting the resulting effects and events. The
testing API consist of essentially three functions: `effects()`, `events()` and
`is_done()`. All three first run the Command's underlying tasks until they
settle and then return an iterator over the accumulated effects or events, and
in the case of `is_done` a bool indicating whether there is any more work to do.

An example test looks like this:

```rust
#[test]
fn request_effect_can_be_resolved() {
    let mut cmd = Command::request_from_shell(AnOperation)
        .then_send(Event::Completed);

    let effect = cmd.effects().next();
    assert!(effect.is_some());

    let Effect::AnEffect(mut request) = effect.unwrap();

    assert_eq!(request.operation, AnOperation);

    request
        .resolve(AnOperationOutput)
        .expect("Resolve should succeed");

    let event = cmd.events().next().unwrap();

    assert_eq!(event, Event::Completed(AnOperationOutput));

    assert!(cmd.is_done())
}
```

In apps, this will be very similar, except the `cmd` will be returned by the
app's `update` function.

This API is mainly for testing, but is available to all consumers in all
contexts, as it can easily become very useful for special cases when composing
applications and virtualising commands in various ways.

### Capabilities are no longer special

With the `Command`, capabilities become command creators and transformers. This
makes them no different from user code in a lot of ways.

The really basic ones can just be a set of functions. Any more complicated ones
can now have state, call other capabilities, transform the commands produced by
them, etc.

The expectation is that the majority of low-level capability APIs will return a
`CommandBuilder`, so that they can be used from both event callback context and
async context equally easily.

### Better app composition

Instead of transforming the app's `Capabilities` types in order to wrap them in
another app up front, when composing apps the resulting commands get
transformed. More specifically, this involves two map calls:

```rust
    let original: Command<Effect, Event> = capability_call();

    let cmd: Command<NewEffect, Event> = original.map_effect(|effect| effect.into()); // provided there's a From impl
    let cmd: Command<Effect, NewEvent> = original.map_event(|event| Event::Child(event));
```

The basic mapping is pretty straightforward, but can become as complex as
required. For example, events produced by a child app can be consumed and
re-routed, duplicated and broadcast across multiple children, etc. The mapping
can also be done by fully wrapping the original Command in another using async

```rust
    let original: Command<Effect, Event> = capability_call();

    let cmd = Command::new(|ctx| async move {
        while let Some(output) = original.next().await {
            match output {
                CommandOutput::Effect(effect) => {
                    // ... do things using `ctx`
                }
                CommandOutput::Event(event) => {
                    // ... do things using `ctx`
                }

            }
        }
    });

```

### Other benefits and features

A grab bag of other things:

- Spawn now returns a `JoinHandle` which can be `.await`ed
- Tasks can be aborted by calling `.abort()` on a `JoinHandle`
- Whole commands can be aborted using an `AbortHandle` returned by
  `.abort_handle()`. The handle can be stored in the model and used later.
- Commands can be "hosted" on a pair of channel senders returning a future which
  should be compatible with the existing executor enabling a reasonably smooth
  migration path
- This API should in theory enable declarative effect middlewares like caching,
  retries, throttling, timeouts, etc...

### Limitations and drawbacks

I'm sure we'll find some. :)

For one, the return type signature for capabilities is not great, for example:
`RequestBuilder<Effect, Event, impl Future<Output = AnOperationOutput>>`.

One major perceived limitation which still remains is that `model` is not
accessible from the effect code. This is by design, to avoid data races from
concurrent access to the model. It should hopefully be a bit more obvious now
that the effect code is _returned_ from the `update` function wrapped in a
Command.

## Open questions and other considerations

- The command API expects the `Effect` type to implement `From<Request<Op>>` for
  any capability Operations it is used with. This is derived by the `Effect`
  macro, and is expected to be supported by a derive macro even in the future
  state.
- We have not fully thought about back-pressure in the Commands (for events,
  effects and spawned tasks) even to the level of "is any needed?"
- We will explore ways to make the code that interleaves effects and state
  updates more "linear" - require fewer intermediate events - separately at a
  later stage
