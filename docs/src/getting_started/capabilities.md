# Adding a capability

The counter's only effect is `Render`. Most apps need more: HTTP, storage,
timers, or something of their own. The
[`counter-http`](https://github.com/redbadger/crux/tree/master/examples/counter-http)
example is the counter with two more effects — it keeps the count on a server,
over HTTP, and hears about changes from other clients through Server-Sent
Events:

```rust,noplayground
// Rust — shared/src/app.rs
{{#include ../../../examples/counter-http/shared/src/app.rs:effect}}
```

The two are different kinds of capability. HTTP comes from `crux_http`, which
ships the shell side of its protocol along with the core side. Server-Sent
Events are this app's own: a stream operation declared in `shared/src/sse.rs`,
which the shell answers with a `Chunk` per batch of bytes it reads, then `Done`:

```rust,noplayground
// Rust — shared/src/sse.rs
{{#include ../../../examples/counter-http/shared/src/sse.rs:operation}}
```

How the core uses them is the subject of Part II, from
[Managed Effects](../part-2/effects.md) to
[Building Capabilities](../part-2/capabilities.md). Here we look at the shell.

## Registering the shipped handler

A capability that ships a shell handler is registered in the codegen, next to
the app:

```rust,noplayground
// Rust — shared/src/bin/codegen.rs
{{#include ../../../examples/counter-http/shared/src/bin/codegen.rs:shell_handler}}
```

`crux_http::HTTP` is only there when `crux_http`'s own `facet_typegen` feature
is on, so the app's `facet_typegen` feature turns it on too:

```toml
# TOML — shared/Cargo.toml
{{#include ../../../examples/counter-http/shared/Cargo.toml:facet_typegen}}
```

`.shell_handler(&crux_http::HTTP)` copies `crux_http`'s handler for each
language into the generated package, and that's all it does. The generated
`EffectHandler` gains an `http` method for the `Http` effect and a
`serverSentEvents` method for the `ServerSentEvents` one whether you register
it or not; registering it means you don't have to write the first one
yourself.

## Implementing the handler

Here's the Swift handler. HTTP is one line, delegating to the shipped
`URLSessionHttpHandler`. Server-Sent Events have no shipped handler, so the
shell implements the stream itself, sending each event it reads into the
`EffectSink` it's given:

```swift
// Swift — apple/CounterApp/CounterHandler.swift
{{#include ../../../examples/counter-http/apple/CounterApp/CounterHandler.swift}}
```

The [Kotlin](https://github.com/redbadger/crux/blob/master/examples/counter-http/Android/app/src/main/java/com/crux/examples/counter/http/CounterHandler.kt)
and [TypeScript](https://github.com/redbadger/crux/blob/master/examples/counter-http/web-nextjs/src/app/core.ts)
handlers have the same shape. Nothing in any of them calls `resolve`: a
request's method returns its output, a stream's method sends into its sink, and
the generated `Core` resolves the request with each.
[Shipped shell handlers](../part-4/typegen.md#shipped-shell-handlers) covers
what the shipped handlers do, and how to configure or replace them.

## Where next

You now have the shape of every Crux app: a core that describes what it wants
done, and a shell that does it, through a handler with a method per operation.

- To see what the generated `Core` does for you, build one by hand in
  [Part I](../part-1/basic_app.md). It builds the same counter step by step,
  writing the loop between the shell and the core itself.
- For depth, continue to [Part II](../part-2/weather_app.md), which builds a
  Weather app with HTTP, key-value storage, timers and location, and covers
  managed effects, testing with effects and building capabilities.
