// Native (iOS/Android) FFI: middleware handles Random effects before they reach the shell.
// The FFI enum has 3 variants (Render, Http, ServerSentEvents).
#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use std::sync::Arc;

    use crux_core::{
        bridge::{EffectId, NativeBridge, NativeBridgeError, ResolveNative},
        middleware::{HandleEffectLayer, Layer as _, MapEffectLayer},
        render::RenderOperation,
        Core, EffectNative, Request,
    };
    use crux_http::protocol::{HttpRequest, HttpResult};

    use crate::{
        middleware::RngMiddleware,
        sse::{SseRequest, SseResponse},
        Counter, Event, ViewModel,
    };

    // --- Effect enum (internal, with Request wrappers) ---

    #[derive(Debug)]
    pub enum Effect {
        Render(Request<RenderOperation>),
        Http(Request<HttpRequest>),
        ServerSentEvents(Request<SseRequest>),
    }

    impl From<Request<RenderOperation>> for Effect {
        fn from(value: Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }

    impl From<Request<HttpRequest>> for Effect {
        fn from(value: Request<HttpRequest>) -> Self {
            Self::Http(value)
        }
    }

    impl From<Request<SseRequest>> for Effect {
        fn from(value: Request<SseRequest>) -> Self {
            Self::ServerSentEvents(value)
        }
    }

    impl From<crate::app::Effect> for Effect {
        fn from(effect: crate::app::Effect) -> Self {
            match effect {
                crate::Effect::Render(request) => Effect::Render(request),
                crate::Effect::Http(request) => Effect::Http(request),
                crate::Effect::ServerSentEvents(request) => Effect::ServerSentEvents(request),
                crate::Effect::Random(_) => panic!("Random effects handled by middleware"),
            }
        }
    }

    impl crux_core::Effect for Effect {}

    // --- FFI types (UniFFI-compatible) ---

    /// The effect operation sent to the shell
    #[derive(Debug, uniffi::Enum)]
    pub enum EffectFfi {
        Render(RenderOperation),
        Http(HttpRequest),
        ServerSentEvents(SseRequest),
    }

    /// The output/response from the shell for each effect type
    #[derive(Debug, uniffi::Enum)]
    pub enum EffectOutput {
        /// Render is a notification effect (fire-and-forget, no response data)
        Render,
        Http(HttpResult),
        ServerSentEvents(SseResponse),
    }

    /// A request sent to the shell with an ID for correlation
    #[derive(Debug, uniffi::Record)]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: EffectFfi,
    }

    // --- EffectNative implementation ---

    impl EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;

        fn into_native(self) -> (Self::Ffi, ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => req.into_native(EffectFfi::Render, |o| match o {
                    EffectOutput::Render => Ok(()),
                    _ => Err(NativeBridgeError::OutputMismatch {
                        expected: "Render".to_string(),
                    }),
                }),
                Effect::Http(req) => req.into_native(EffectFfi::Http, |o| match o {
                    EffectOutput::Http(v) => Ok(v),
                    _ => Err(NativeBridgeError::OutputMismatch {
                        expected: "Http".to_string(),
                    }),
                }),
                Effect::ServerSentEvents(req) => {
                    req.into_native(EffectFfi::ServerSentEvents, |o| match o {
                        EffectOutput::ServerSentEvents(v) => Ok(v),
                        _ => Err(NativeBridgeError::OutputMismatch {
                            expected: "ServerSentEvents".to_string(),
                        }),
                    })
                }
            }
        }
    }

    /// Shell-facing events (only the variants the shell can send)
    #[derive(uniffi::Enum)]
    pub enum EventFfi {
        Get,
        Increment,
        Decrement,
        Random,
        StartWatch,
    }

    impl From<EventFfi> for Event {
        fn from(event: EventFfi) -> Self {
            match event {
                EventFfi::Get => Event::Get,
                EventFfi::Increment => Event::Increment,
                EventFfi::Decrement => Event::Decrement,
                EventFfi::Random => Event::Random,
                EventFfi::StartWatch => Event::StartWatch,
            }
        }
    }

    /// Shell callback for receiving typed effects
    #[uniffi::export(with_foreign)]
    pub trait NativeShell: Send + Sync {
        fn handle_effect(&self, request: NativeRequest);
    }

    /// Core FFI using `NativeBridge` for typed effects
    #[derive(uniffi::Object)]
    pub struct CoreFFI {
        bridge:
            NativeBridge<MapEffectLayer<HandleEffectLayer<Core<Counter>, RngMiddleware>, Effect>>,
    }

    #[uniffi::export]
    #[allow(clippy::missing_panics_doc)]
    impl CoreFFI {
        #[uniffi::constructor]
        pub fn new(shell: Arc<dyn NativeShell>) -> Self {
            // Middleware invariant: only pure-Rust effects (RNG, crypto, local compute)
            // should be handled here. Effects with observable side effects (network, storage)
            // must always reach the shell.
            let bridge = Core::<Counter>::new()
                .handle_effects_using(RngMiddleware::new())
                .map_effect::<Effect>()
                .native_bridge(move |id, effect| {
                    shell.handle_effect(NativeRequest { id: id.0, effect });
                });

            Self { bridge }
        }

        pub fn update(&self, event: EventFfi) {
            self.bridge
                .update(Event::from(event))
                .expect("update failed");
        }

        pub fn resolve(&self, id: u32, output: EffectOutput) {
            self.bridge
                .resolve(EffectId(id), output)
                .expect("resolve failed");
        }

        #[must_use]
        pub fn view(&self) -> ViewModel {
            self.bridge.view()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crossbeam_channel::{self, Receiver};
        use crux_http::protocol::{HttpResponse, HttpResult};
        use std::time::Duration;

        struct MockShell {
            tx: crossbeam_channel::Sender<NativeRequest>,
        }

        impl NativeShell for MockShell {
            fn handle_effect(&self, request: NativeRequest) {
                self.tx.send(request).unwrap();
            }
        }

        fn create_core() -> (CoreFFI, Receiver<NativeRequest>) {
            let (tx, rx) = crossbeam_channel::unbounded();
            let core = CoreFFI::new(Arc::new(MockShell { tx }));
            (core, rx)
        }

        /// Drain all currently available effects from the channel.
        fn drain(rx: &Receiver<NativeRequest>) -> Vec<NativeRequest> {
            rx.try_iter().collect()
        }

        #[test]
        fn get_roundtrip() {
            let (core, rx) = create_core();

            // Default view before any events
            assert_eq!(core.view().text, "0 (pending)");
            assert!(!core.view().confirmed);

            // Get → Http effect
            core.update(EventFfi::Get);
            let effects = drain(&rx);
            assert_eq!(effects.len(), 1);
            let EffectFfi::Http(ref op) = effects[0].effect else {
                panic!("expected Http effect, got {:?}", effects[0].effect);
            };
            assert!(op.url.contains("crux-counter.fly.dev"));

            // Resolve Http → Set → Update → Render
            core.resolve(
                effects[0].id,
                EffectOutput::Http(HttpResult::Ok(
                    HttpResponse::ok()
                        .body(r#"{"value": 42, "updated_at": 1672531200000}"#)
                        .build(),
                )),
            );

            let effects = drain(&rx);
            assert_eq!(effects.len(), 1);
            assert!(
                matches!(effects[0].effect, EffectFfi::Render(_)),
                "expected Render, got {:?}",
                effects[0].effect
            );

            assert_eq!(core.view().text, "42 (2023-01-01 00:00:00 UTC)");
            assert!(core.view().confirmed);
        }

        #[test]
        fn increment_roundtrip() {
            let (core, rx) = create_core();

            // Increment → Render (optimistic) + Http
            core.update(EventFfi::Increment);
            let effects = drain(&rx);
            assert_eq!(effects.len(), 2, "expected Render + Http effects");

            assert!(
                effects
                    .iter()
                    .any(|e| matches!(e.effect, EffectFfi::Render(_))),
                "expected a Render effect for optimistic update"
            );
            let http_req = effects
                .iter()
                .find(|e| matches!(e.effect, EffectFfi::Http(_)))
                .expect("expected an Http effect");

            // View shows optimistic update
            assert_eq!(core.view().text, "1 (pending)");
            assert!(!core.view().confirmed);

            // Resolve Http → confirmed state
            core.resolve(
                http_req.id,
                EffectOutput::Http(HttpResult::Ok(
                    HttpResponse::ok()
                        .body(r#"{"value": 1, "updated_at": 1672531200000}"#)
                        .build(),
                )),
            );

            let effects = drain(&rx);
            assert_eq!(effects.len(), 1);
            assert!(matches!(effects[0].effect, EffectFfi::Render(_)));

            assert_eq!(core.view().text, "1 (2023-01-01 00:00:00 UTC)");
            assert!(core.view().confirmed);
        }

        #[test]
        fn random_middleware_intercepts() {
            let (core, rx) = create_core();

            // Random → middleware intercepts on bg thread, resolves asynchronously.
            // No Random variant exists in native EffectFfi (compile-time guarantee).
            core.update(EventFfi::Random);

            // Wait for the middleware's bg thread to resolve.
            // Random value in [-5, 5]: produces Render (always) + Http (if non-zero).
            let first = rx
                .recv_timeout(Duration::from_secs(5))
                .expect("timed out waiting for middleware to resolve Random effect");

            assert!(
                matches!(first.effect, EffectFfi::Render(_)),
                "expected Render from middleware chain, got {:?}",
                first.effect
            );
        }
    }
}

// WASM FFI: no middleware (threads unavailable), Random is a shell effect.
// The FFI enum has 4 variants (Render, Http, ServerSentEvents, Random).
#[cfg(target_arch = "wasm32")]
pub mod native {
    use std::sync::Arc;

    use crux_core::{
        bridge::{EffectId, NativeBridge, NativeBridgeError, ResolveNative},
        middleware::{Layer as _, MapEffectLayer},
        render::RenderOperation,
        Core, EffectNative, Request,
    };
    use crux_http::protocol::{HttpRequest, HttpResult};

    use crate::{
        capabilities::{RandomNumber, RandomNumberRequest},
        sse::{SseRequest, SseResponse},
        Counter, Event, ViewModel,
    };

    // --- Effect enum (internal, with Request wrappers) ---

    #[derive(Debug)]
    pub enum Effect {
        Render(Request<RenderOperation>),
        Http(Request<HttpRequest>),
        ServerSentEvents(Request<SseRequest>),
        Random(Request<RandomNumberRequest>),
    }

    impl From<Request<RenderOperation>> for Effect {
        fn from(value: Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }

    impl From<Request<HttpRequest>> for Effect {
        fn from(value: Request<HttpRequest>) -> Self {
            Self::Http(value)
        }
    }

    impl From<Request<SseRequest>> for Effect {
        fn from(value: Request<SseRequest>) -> Self {
            Self::ServerSentEvents(value)
        }
    }

    impl From<Request<RandomNumberRequest>> for Effect {
        fn from(value: Request<RandomNumberRequest>) -> Self {
            Self::Random(value)
        }
    }

    impl From<crate::app::Effect> for Effect {
        fn from(effect: crate::app::Effect) -> Self {
            match effect {
                crate::Effect::Render(request) => Effect::Render(request),
                crate::Effect::Http(request) => Effect::Http(request),
                crate::Effect::ServerSentEvents(request) => Effect::ServerSentEvents(request),
                crate::Effect::Random(request) => Effect::Random(request),
            }
        }
    }

    impl crux_core::Effect for Effect {}

    // --- FFI types (UniFFI-compatible) ---

    /// The effect operation sent to the shell
    #[derive(Debug, uniffi::Enum)]
    pub enum EffectFfi {
        Render(RenderOperation),
        Http(HttpRequest),
        ServerSentEvents(SseRequest),
        Random(RandomNumberRequest),
    }

    /// The output/response from the shell for each effect type
    #[derive(Debug, uniffi::Enum)]
    pub enum EffectOutput {
        /// Render is a notification effect (fire-and-forget, no response data)
        Render,
        Http(HttpResult),
        ServerSentEvents(SseResponse),
        Random(RandomNumber),
    }

    /// A request sent to the shell with an ID for correlation
    #[derive(Debug, uniffi::Record)]
    pub struct NativeRequest {
        pub id: u32,
        pub effect: EffectFfi,
    }

    // --- EffectNative implementation ---

    impl EffectNative for Effect {
        type Ffi = EffectFfi;
        type Output = EffectOutput;

        fn into_native(self) -> (Self::Ffi, ResolveNative<Self::Output>) {
            match self {
                Effect::Render(req) => req.into_native(EffectFfi::Render, |o| match o {
                    EffectOutput::Render => Ok(()),
                    _ => Err(NativeBridgeError::OutputMismatch {
                        expected: "Render".to_string(),
                    }),
                }),
                Effect::Http(req) => req.into_native(EffectFfi::Http, |o| match o {
                    EffectOutput::Http(v) => Ok(v),
                    _ => Err(NativeBridgeError::OutputMismatch {
                        expected: "Http".to_string(),
                    }),
                }),
                Effect::ServerSentEvents(req) => {
                    req.into_native(EffectFfi::ServerSentEvents, |o| match o {
                        EffectOutput::ServerSentEvents(v) => Ok(v),
                        _ => Err(NativeBridgeError::OutputMismatch {
                            expected: "ServerSentEvents".to_string(),
                        }),
                    })
                }
                Effect::Random(req) => req.into_native(EffectFfi::Random, |o| match o {
                    EffectOutput::Random(v) => Ok(v),
                    _ => Err(NativeBridgeError::OutputMismatch {
                        expected: "Random".to_string(),
                    }),
                }),
            }
        }
    }

    /// Shell-facing events (only the variants the shell can send)
    #[derive(uniffi::Enum)]
    pub enum EventFfi {
        Get,
        Increment,
        Decrement,
        Random,
        StartWatch,
    }

    impl From<EventFfi> for Event {
        fn from(event: EventFfi) -> Self {
            match event {
                EventFfi::Get => Event::Get,
                EventFfi::Increment => Event::Increment,
                EventFfi::Decrement => Event::Decrement,
                EventFfi::Random => Event::Random,
                EventFfi::StartWatch => Event::StartWatch,
            }
        }
    }

    /// Shell callback for receiving typed effects
    #[uniffi::export(with_foreign)]
    pub trait NativeShell: Send + Sync {
        fn handle_effect(&self, request: NativeRequest);
    }

    /// Core FFI using `NativeBridge` for typed effects (no middleware on WASM)
    #[derive(uniffi::Object)]
    pub struct CoreFFI {
        bridge: NativeBridge<MapEffectLayer<Core<Counter>, Effect>>,
    }

    #[uniffi::export]
    #[allow(clippy::missing_panics_doc)]
    impl CoreFFI {
        #[uniffi::constructor]
        pub fn new(shell: Arc<dyn NativeShell>) -> Self {
            let bridge =
                Core::<Counter>::new()
                    .map_effect::<Effect>()
                    .native_bridge(move |id, effect| {
                        shell.handle_effect(NativeRequest { id: id.0, effect });
                    });

            Self { bridge }
        }

        pub fn update(&self, event: EventFfi) {
            self.bridge
                .update(Event::from(event))
                .expect("update failed");
        }

        pub fn resolve(&self, id: u32, output: EffectOutput) {
            self.bridge
                .resolve(EffectId(id), output)
                .expect("resolve failed");
        }

        #[must_use]
        pub fn view(&self) -> ViewModel {
            self.bridge.view()
        }
    }
}
