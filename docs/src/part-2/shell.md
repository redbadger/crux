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

Those are the six possible variants we'll see in the return from `update`. It
is essentially telling us "I did the state update, and here are some side-effects
for you to perform".

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

Let's look at how this works in practice.

## Platforms

You can continue with your platform of choice:

- [iOS/macOS with SwiftUI](./shell/ios.md)
- [Android with Jetpack Compose](./shell/android.md)
- [Web with Leptos](./shell/leptos.md)
- [Web with React](./shell/react.md)
