# Capabilities

In the last chapter, we spoke about Effects. In this one we'll look at the APIs
your app will actually use to request them – the capabilities.

Capabilities are reusable, platform agnostic APIs for a particular type of
effect. They have two key jobs:

1. Provide a nice ergonomic API for apps to use to create `Command`s
2. Manage the communication between the app and the Shell by defining an `Operation` type

From the perspective of the app, you can think of capabilities as an equivalent
to SDKs. And a lot of them will provide an interface to the actual platform
specific SDKs.

There is not a lot more to be said about using capabilities, your best source is probably
the [documentation of the provided Time capability](https://docs.rs/crux_time/latest/crux_time/command/struct.Time.html), you'll find
that the basic capabilities just define some types and a few functions.

If you're interested in building your own capability, read the [next chapter](./capability_apis.md).

----

```admonish warning title="The rest of this page is deprecated!"
You can safely continue with the [next chapter](./capability_apis.md).

Capabilities required a lot more explanation, most of which has moved to
the [Managed Effects](./effects.md) chapter, which you should follow,
unless you're working on an older app, which still defines a `Capabilities` type.

The rest of this page covers the older approach, which you should consider
deprecated.
```

## Using Capabilities

Okay, time to get practical. We'll look at what it takes (and why) to use a
capability, and in the next couple of chapters, we'll continue to build one and
implement the Shell side of it.

Firstly, we need to have access to an instance of the capability in our `update`
function. Recall that the function signature is:

```rust,noplayground
fn update(&self, msg: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities)
```

We get the capabilities in the `caps` argument. You may be wondering why that's
necessary. At first glance, we could be able to just create a capability
instance ourselves, or not need one at all, after all they just provide API to
make effects. There are a few reasons.

Firstly, capabilities need to be able to send a message to the shell, more
precisely, they need to be able to add to the set of effects which result from
the run of the update function. Sounds like a return value to you? It kind of
is, and we tried that, and the type signatures involved quickly become quite
unsightly. It's not the only reason though. They also need to be able to return
information back to your app by queuing up events to be dispatched to the next
run of the `update` function. But to be _really_ useful, they need to be able to
do a series of these things and suspend their execution in the meantime.

In order to enable all that, Crux needs to be in charge of creating the instance
of the capabilities to provide context to them, which they use to do the things
we just listed. We'll see the details of this in the next chapter.

Notice that the type of the argument is `Self::Capabilities` — you own the type.
This is to allow you to declare which capabilities you want to use in your app.
That type will most likely be a struct looking like the following:

```rust,noplayground
#[derive(Effect)]
pub struct Capabilities {
    pub http: Http<Event>,
    pub render: Render<Event>,
}
```

Those two types come from `crux_core` and `crux_http`. Two things are suspicious
about the above — the `Event` type, which describes your app's events and the
`#[derive(Effect)]` derive macro.

The latter generates an `Effect` enum for you, used as the payload of the
messages to the Shell. It is one of the things you will need to expose via the
FFI boundary. It's the type the Shell will use to understand what is being
requested from it, and it mirrors the `Capabilities` struct: for each field,
there is a tuple variant in the Effect enum, with the respective capability's
_request_ as payload, i.e. the data describing what's being asked of the Shell.

The `Event` type argument enables the "shell side" of these capabilities to send
you your specific events back as the _outcome_ of their work. Typically, you'd
probably set up an `Event` variant specifically for the individual uses of each
capability, like this:

```rust,noplayground
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Event {
    Hello,
    #[serde(skip)]
    Set(crux_http::Result<crux_http::Response<Counter>>), // <- this
}
```

In a real app, you'd likely have more than one interaction with a HTTP server,
and would most likely need one variant for each. (`#[serde(skip)]` in the above
code hides the variant from the type exposed to the Shell for direct calls –
this event should not be dispatched directly. The other reason for it also has
to do with serialization difficulties, which we'll eventually iron out).

That's it for linking the capability into our app, now we can use it in the
`update` function:

```rust,noplayground
    fn update(&self, msg: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match msg {
            Event::Get => {
                caps.http
                    .get(API_URL)
                    .expect_json::<Counter>()
                    .send(Event::Set);

                caps.render.render();
            }
    // ...
```

You can see the use of the `Event::Set` variant we just discussed. `Event::Set`
is technically a function with this signature:

```rust,noplayground
fn Event::Set(crux_http::Result<crux_http::Response<Counter>) -> Event
```

Looks a lot like a callback, doesn't it. Yep. With the difference that the
result is an `Event`. Generally, you should be able to completely ignore this
detail and just use your variant names and the code should read pretty clearly:
"When done, send me `Event::Set`".

The other nuance to be aware of is that the capability calls return immediately.
This should hopefully be relatively obvious by now, but all that's happening is
effects are getting queued up to be requested from the Shell. In a way,
capability calls are implicitly asynchronous (but you can't await them).

That's generally all there is to it. What you'll notice is that most
capabilities have essentially request/response semantics — you use their APIs,
and provide an event you want back, and eventually your update function will get
called with that event. Most capabilities take inputs for their effect, and
return output in their outcomes, but some capabilities don't do one or either of
those things. Render is an example of a capability which doesn't take payload
and never calls back. You'll likely see all the different variations in Crux
apps.

## Orchestrating capability calls

In more complex apps, you might run into situations where you need to run
several effects in parallel, race them, run them in sequence or a combination of
the above. In other words, in some scenarios, you really need the full control
of `async`/`await` and the futures APIs.

To support this case, Crux provides a built-in capability called `Compose`,
which provides restricted but direct access to the capability runtime (more
about the runtime in the next chapter), which supports `async`. To use it, first
add it to your Capabilities struct:

```rust,noplayground
use crux::compose::Compose;

#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    pub http: Http<Event>,
    pub render: Render<Event>,
    #[effect(skip)] // skips the compose variant when deriving an Effect enum
    pub compose: Compose<Event>,
}
```

Then, you can use it in your update function like this:

```rust,noplayground
fn update(&self, msg: Event, model: &mut Model, caps: &Capabilities) {
    match msg {
        Event::GetDocuments => caps.compose.spawn(|context| {
            let http = caps.http.clone();

            async move {
                let ids = http
                    .get(DOCS_URL)
                    .await
                    .expect("Request should send")
                    .body_json::<Vec<Id>>()
                    .await
                    .expect("Ids failed to parse as JSON");

                let futs: Vec<_> = ids
                    .iter()
                    .map(|id| {
                        let http = http.clone();

                        async move {
                            http.get(&format!("{}/{}", DOCS_URL, id))
                                .await
                                .expect("request did not send")
                                .body_json::<Doc>()
                                .await
                                .expect("doc failed to parse as JSON")
                        }
                    })
                    .collect();

                let docs = futures::future::join_all(futs).await;

                context.update_app(Event::FetchedDocuments(docs))
            }
        }),
        // ...
    }
}
```

The above code first fetches a list of document IDs, then fetches each document
in parallel, and finally returns the list of documents as an event.

The `spawn` method takes a closure which is passed a `ComposeContext` argument.
This is a handle to the capability runtime, which allows you to send events back
to the app. The closure must return a future, which is then spawned on the
runtime. The runtime will drive the future to completion. You can call
`context.update_app` multiple times if necessary.

One consideration of this style of orchestration is that the more effects you
string together this way, the harder it will be to test the behaviour of this
ad-hoc capability, because you can't start the transaction in the middle.
Generally, if you find yourself sending events using `update_app` and then
continuing to emit more effects, you should probably break the orchestration up
into smaller blocks executed in response to the events in the update function
instead.

Now that we know how to use capabilities, we're ready to look at building our
own ones. You may never need to do that, or it might be one of the first hurdles
you'll come across (and if we're honest, given how young Crux is, it's more
likely the latter). Either way, it's what we'll do in the next chapter.
