pub mod native {
    use std::sync::Arc;

    use crux_core::{
        Core, EffectNative, Request,
        bridge::{EffectId, NativeBridge, NativeBridgeError, ResolveNative, UnitOutput},
        middleware::{HandleEffectLayer, Layer as _, MapEffectLayer},
        render::RenderOperation,
    };
    use crux_http::protocol::{HttpRequest, HttpResult};

    use crate::{
        Counter, Event, ViewModel,
        middleware::RngMiddleware,
        sse::{SseRequest, SseResponse},
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
        /// Render has no output (uses UnitOutput for UniFFI compatibility)
        Render(UnitOutput),
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
                    EffectOutput::Render(_) => Ok(UnitOutput),
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

    /// Core FFI using NativeBridge for typed effects
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
}

// Note: wasm_bindgen and wit_bindgen modules removed.
// uniffi-bindgen-react-native generates TypeScript bindings for BOTH:
// - React Native (iOS/Android via JSI)
// - Web pages (via generated WASM crate)
// from the same UniFFI annotations in the `native` module above.
