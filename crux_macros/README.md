# Crux Macros

This crate provides macros that can be used in conjunction with [`crux_core`](https://crates.io/crates/crux_core) and associated Capabilities.

## Effect

The `Effect` derive macro can be used to create an effect enum for use by the Shell when performing side effects for the Core. It also derives `WithContext` for the Capabilities struct.

The name of the generated enum is `Effect` by default, but can be overridden (see below). The macro needs to know the name of your app struct (which is `App` by default, but can be specified), and your event enum type (`Event` by default, but can be specified).

It also needs to know the operation types for any capabilities you are using that have non-unit structs as their request types (have the `Operation` trait implemented).

> The macro should really, by convention, be called `WithContext`, but it's possible that the name `Effect` more usefully describes the code that is generated.

> It is implemented as a derive macro, rather than an attribute macro, because it needs to be configured by non-attribute macros within the struct itself (which is not possible with attribute macros).

### Example usage

If you want to generate an enum called `Effect` and your app struct is called `App`, and your event enum is called `Event`, and the capabilities you are using only have unit operations, then you can simply just use the macro without additional configuration:

```rust
#[derive(Effect)]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

If, however, you are using capabilities with non-unit operations (i.e. that take request data), you have to tell the macro what the operation struct is called (this is the struct that implements the `Operation` trait):

```rust
#[derive(Effect)]
pub struct Capabilities {
    #[effect(operation = "HttpRequest")]
    pub http: Http<Event>,
    pub render: Render<Event>,
}
```

If you want the generated Effect enum to be called something different, you can specify another name:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

If your app struct (that implements the `App` trait) is called something other than `App`, you can specify its name:

```rust
#[derive(Effect)]
#[effect(app = "MyApp")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

If your event enum (that the Shell uses to send events to the Core) is called something other than `Event`, you can specify its name:

```rust
#[derive(Effect)]
#[effect(event = "MyEvent")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

To specify both `app` _and_ `name` you can either use the attribute twice, like this:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect")]
#[effect(app = "MyApp")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

Or, more idiomatically, combine them into one usage, like this:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect", app = "MyApp")]
pub struct Capabilities {
    pub render: Render<Event>,
}
```

Full usage might look something like this:

```rust
#[derive(Effect)]
#[effect(name = "MyEffect", app = "MyApp", event = "MyEvent")]
pub struct CatFactCapabilities {
    #[effect(operation = "HttpRequest")]
    pub http: Http<Event>,
    #[effect(operation = "KeyValueOperation")]
    pub key_value: KeyValue<Event>,
    pub platform: Platform<Event>,
    pub render: Render<Event>,
    pub time: Time<Event>,
}
```
