# FFI bridge

In the previous chapter, we saw how the capability runtime facilitates the
orchestration of effect processing by the shell. We looked at the simpler
scenario where the shell was built in Rust. Now we'll extend this to the more
common scenario where the shell is written in a different language and the core
APIs are called over a Foreign Function Interface, passing events, requests and
responses back and forth, serialised as bytes.

## The FFI bridge

The FFI bridge has two key parts, the serialisation part converting from typed
effect requests to serializable types, and the FFI implementation itself,
facilitated by [UniFFI](https://github.com/mozilla/uniffi-rs).

The serialisation part is facilitated by the `Bridge`. It is a wrapper for the
`Core` with its own definition of `Request`. Its API is very similar to the
`Core` API - it has an identical set of methods, but their type signatures are
different.

For example, here is `Core::resolve`

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/mod.rs:resolve_sig}}
```

and here's its counterpart, `Bridge::handle_response`

```rust,no_run,noplayground
{{#include ../../../crux_core/src/bridge/mod.rs:handle_response_sig}}
```

where the core expects to be given a `Request<Op>` to resolve, the bridge
expects a `id` - a unique identifier of the request being resolved.

This makes sense - the `Request`s include callback closures working with the
capability runtime, they can't be easily serialised and sent back and forth
across the language boundary. Instead, the bridge "parks" them in a registry, to
be picked up later. Like in a theatre cloakroom, the registry returns a unique
number under which the request is stored.

The implementation of the serialization/deserialization process is slightly
complicated by the fact that Crux allows you to supply your own serializer and
deserializer should you need to, so the actual bridge implementation does not
work on bytes but on serializers. The `Bridge` type used in examples and all the
documentation is a default implementation, which uses bincode serialization,
which is also supported by the [type generation subsystem](./typegen.md).

We won't go into the detail of working with Serde and the
[`erased_serde`](https://docs.rs/erased-serde/) crate to make all the
serialization happen without leaking deserialization lifetimes out of the
bridge. You can read the implementation of `BridgeWithSerializer` if you're
interested in the gory details. For our purposes, the type definition will
suffice.

```rust,no_run,noplayground
{{#include ../../../crux_core/src/bridge/mod.rs:bridge_with_serializer}}
```

The bridge holds an instance of the `Core` and a `ResolveRegistry` to store the
effect requests in.

The processing of the update loop is quite similar to the Core update loop:

- When a serialized event arrives, it is deserialized and passed to the `Core`'s
  `process_event`
- When a request response arrives, its id is forwarded to the
  `ResolveRegistry`'s `resume` method, and the `Core`'s `process` method is
  called to run the capability runtime

You may remember that both these calls return effect requests. The remaining
step is to store these in the registry, using the registry's `register` method,
exchanging the core `Request` for a bridge variant, which looks like this:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/bridge/mod.rs:request}}
```

Unlike the core request, this does not include any closures and is fully
serializable.

## ResolveRegistry

It is worth pausing for a second on the resolve registry. There is one tricky
problem to solve here - storing the generic `Request`s in a single store. We get
around this by making the `register` method generic and asking the effect to
"serialize" itself.

```rust,no_run,noplayground
{{#include ../../../crux_core/src/bridge/registry.rs:register}}
```

this is named based on our intent, not really based on what actually happens.
The method comes from an `Effect` trait:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/core/effect.rs:effect}}
```

Like the `Effect` type which implements this trait, the implementation is macro
generated, based on the Capabilities used by your application. We will look at
how this works in the [`Effect type`](./effect.md) chapter.

The type signature of the method gives us a hint though - it converts the normal
`Effect` into a serializable counterpart, alongside something with a
`ResolveSerialized` type. This is stored in the registry under an id, and the effect and the id are returned as the bridge version
of a `Request`.

The definition of the `ResolveSerialized` type is a little bit convoluted:

```rust,no_run,noplayground
{{#include ../../../crux_core/src/bridge/request_serde.rs:resolve_serialized}}
```

but the gist of it is that it is a mirror of the `Resolve` type we already know,
except it takes a Deserializer. More about this serialization trickery in the
next chapter.

## FFI interface

The final piece of the puzzle is the FFI interface itself. All it does is expose
the bridge API we've seen above.

```admonish note
You will see that this part, alongside the type generation, is a fairly
complicated constellation of various existing tools and libraries, which has
a number of rough edges. It is likely that we will explore replacing this
part of Crux with a tailor made FFI bridge in the future. If/when we do, we will
do our best to provide a smooth migration path.
```

Here's a typical app's shared crate `src/lib.rs` file:

```rust,no_run,noplayground
{{#include ../../../examples/bridge_echo/shared/src/lib.rs}}
```

Ignore the TODO, we will get to that eventually, I promise. There are two forms
of FFI going on - the `wasm_bindgen` annotations on the three functions,
exposing them when built as webassembly, and also the line saying

```rust,no_run,noplayground
uniffi::include_scaffolding!("shared");
```

which refers to the `shared.udl` file in the same folder

```
{{#include ../../../examples/bridge_echo/shared/src/shared.udl}}
```

This is UniFFI's interface definition used to generate the scaffolding for the
FFI interface - both the externally callable functions in the `shared` library,
and their counterparts in the "foreign" languages (like Swift or Kotlin).

The scaffolding is built in the `build.rs` script of the crate

```rust,no_run,noplayground
{{#include ../../../examples/bridge_echo/shared/build.rs}}
```

The foreign language code is built by an additional binary target for the same
crate, in `src/bin/uniffi-bindgen.rs`

```rust,no_run,noplayground
{{#include ../../../examples/bridge_echo/shared/src/bin/uniffi-bindgen.rs}}
```

this builds a CLI which can be used as part of the build process for clients of
the library to generate the code.

The details of this process are well documented
[in UniFFI's tutorial](https://mozilla.github.io/uniffi-rs/Getting_started.html).
