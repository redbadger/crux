# Crux Macros

This crate provides three derive macros (`Effect`, `Export` and `Capability`)
that can be used in conjunction with
[`crux_core`](https://crates.io/crates/crux_core) and associated (or custom)
Capabilities.

## 1. Effect

The `Effect` derive macro can be used to create an effect enum for use by the
Shell when performing side effects for the Core. It also derives `WithContext`
for the Capabilities struct.

The name of the generated enum is `Effect` by default, but can be overridden
(see below). The macro also needs to know the name of your app struct (which is
`App` by default, but can be specified).

It also needs to know the operation types for any capabilities you are using
that have non-unit structs as their request types (have the `Operation` trait
implemented).

> The macro should really, by convention, be called `WithContext`, but it's
> possible that the name `Effect` more usefully describes the code that is
> generated.

> It is implemented as a derive macro, rather than an attribute macro, because
> it needs to be configured by non-macro attributes within the struct itself
> (which is not possible with attribute macros).

### Example usage

If you want to generate an enum called `Effect` and your app struct is called
`App`, and the capabilities you are using only have unit operations, then you
can simply just use the macro without additional configuration:

```rust
#[derive(Effect)]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

If you want the generated Effect enum to be called something different, you can
specify another name:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

If your app struct (that implements the `App` trait) is called something other
than `App`, you can specify its name:

```rust
#[derive(Effect)]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

To specify both `app` _and_ `name` you can either use the attribute twice, like
this:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

Or, more idiomatically, combine them into one usage, like this:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

Full usage might look something like this:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect")]
pub struct CatFactCapabilities {
    pub http: Http<MyEvent>,
    pub key_value: KeyValue<MyEvent>,
    pub platform: Platform<MyEvent>,
    pub render: Render<MyEvent>,
    pub time: Time<MyEvent>,
}
```

## 2. Export

The `Export` derive macro generates code to register the types used by your
capabilities, during foreign type generation in your `shared_types` library.

To use it, declare a feature `typegen` in your `shared` crate, and then annotate
your `Capabilities` struct with the `Export` derive macro:

```rust
#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(Effect)]
pub struct Capabilities {
    pub render: Render<Event>,
    pub http: Http<MyEvent>,
    //...
}
```

Then, in the `build.rs` file of your `shared_types` crate, when you register
your `App`, the types used by your capabilities will also be registered:

```rust
use crux_core::typegen::TypeGen;
use shared::{App, Event};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<App>().expect("register");

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))
        .expect("swift type gen failed");

    gen.java("com.example.counter.shared_types", output_root.join("java"))
        .expect("java type gen failed");

    gen.typescript("shared_types", output_root.join("typescript"))
        .expect("typescript type gen failed");
}
```

## 3. Capability

The `Capability` derive macro can be used to implement the `Capability` trait
when writing your own capabilities. It generates code similar to the following:

```rust
impl<Ev> crux_core::capability::Capability<Ev> for Render<Ev> {
    type Operation = RenderOperation;
    type MappedSelf<MappedEv> = Render<MappedEv>;
    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Render::new(self.context.map_event(f))
    }
}
```

This allows you to derive an instance of a capability from an existing one and
adapt it to a different Event type, which is useful when composing Crux apps
from smaller Crux apps, automatically wrapping the child events in the parent
events.
