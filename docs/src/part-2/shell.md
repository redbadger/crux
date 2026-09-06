# The shell

We've looked at how the Weather app's core fits together, how it's structured into nested state machines, and how managed effects make it testable end-to-end. Time to build the UI around it.

(In practice, you wouldn't write the whole core before touching the UI — you'd go feature by feature. But the shape is the same: a tested core first, then a shell that drives it and handles its effects.)

The shell will have two responsibilities:

1. Laying out the UI components, like we've already seen in Part I
2. Supporting the app's capabilities. This will be new to us

Like in Part I, you can choose which Shell language you'd like to see this in, but first let's talk about
what they all have in common.

## Message interface between core and shell

In Part I, we learned to use the `update` and `view` APIs of the core. We also learned that
in their raw form, they take serialized values as byte buffers.

We skimmed over the return value of `update` very quickly. In that case it only ever
returned a request for a `RenderOperation` - a signal that a new view model is available.

In the Weather's case, more options are possible. Recall the effect type:

```rust
{{#include ../../../examples/weather/shared/src/effects/mod.rs:effect}}
```

Those are the eleven possible variants we'll see in the return from `update` —
one per operation the app can ask for. It is essentially telling us "I did the
state update, and here are some side-effects for you to perform".

Let's say that the effect is an HTTP request. We execute it, get a response, and
what do we do then? Well, that's what the third core API, `resolve`, is for:

```rust
pub fn update(data: &[u8]) -> Vec<u8>
pub fn resolve(id: u32, data: &[u8]) -> Vec<u8>
pub fn view() -> Vec<u8>
```

Each effect request comes with an identifier. We use `resolve` to return the
output of the effect back to the app, alongside the identifier, so that it can
be paired correctly.

## How many times to resolve

`resolve` raises a question the shell has to get right, and the answer is not
in the bytes: how many times does *this* effect get resolved?

- Some effects are **notifications**. `Render` is the obvious one — the core is
  telling the shell something and is not waiting for an answer. Resolving one
  is an error, because the core kept no record of the request.
- Most are **requests**, resolved exactly once, with the operation's output.
- Some are **streams**, resolved once per item, for as long as the subscription
  lives.

Because each operation declares its kind in Rust, [type generation](../part-4/typegen.md)
can tell the shell. Every generated `Effect` gains a `requestKind` accessor
(`effectRequestKind(effect)` in TypeScript), and — more usefully — an
`EffectHandler` protocol/interface with one method per variant, whose signature
*is* the answer:

- a notification's method returns nothing;
- a request's method returns the operation's output (`async` / `suspend` /
  `Promise` / `Task`), and the generated `EffectDispatcher` resolves the
  request with it, once;
- a stream's method is handed an `EffectSink<Output>`, and each `send` resolves
  the request again.

A shell that implements the handler and lets `EffectDispatcher` do the
resolving cannot resolve the wrong number of times or with the wrong type,
because there is no `resolve` call left for it to get wrong. Its `resolve`
argument is the shell's own callback around the core's `resolve` FFI.

Three of the shells that follow do exactly that. The Leptos shell doesn't:
core and shell are both Rust there, so it matches on the `Effect` enum
directly, which is just as precise and needs no generated code. Matching by
hand is still supported everywhere — the generated handler API is additive.

Let's look at how this works in practice.

## Platforms

You can continue with your platform of choice:

- [iOS/macOS with SwiftUI](./shell/ios.md)
- [Android with Jetpack Compose](./shell/android.md)
- [Web with Leptos](./shell/leptos.md)
- [Web with React](./shell/react.md)
