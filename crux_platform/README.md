# crux_platform (deprecated)

> **⚠️ This crate is deprecated and will no longer be maintained.**

The `crux_platform` capability was provided as a convenience for querying the current platform
(iOS, Android, Web, etc.) from the shell. In practice it never did much, and with the
[Command API](https://docs.rs/crux_core/latest/crux_core/command/index.html) it is
straightforward to create your own platform capability tailored to your app's needs.

## Migration

Copy the handful of types below into your own project and adjust them however you like.
The shell-side implementation (responding to `PlatformRequest` with a `PlatformResponse`)
remains exactly the same.

```rust
use crux_core::{Command, Request, command::RequestBuilder, capability::Operation};
use facet::Facet;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Facet, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformRequest;

#[derive(Facet, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}

pub struct Platform<Effect, Event> {
    _marker: PhantomData<(Effect, Event)>,
}

impl<Effect, Event> Platform<Effect, Event>
where
    Effect: From<Request<PlatformRequest>> + Send + 'static,
    Event: Send + 'static,
{
    pub fn get() -> RequestBuilder<Effect, Event, impl Future<Output = PlatformResponse>> {
        Command::request_from_shell(PlatformRequest)
    }
}
```

For more information on building custom capabilities, see
[Managed Effects](https://redbadger.github.io/crux/guide/effects.html) in the Crux book.
