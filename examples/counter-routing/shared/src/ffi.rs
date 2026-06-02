#![allow(clippy::used_underscore_items)]

use crux_core::{Core, bridge::EffectId};

use crate::Counter;

// ── Non-wasm imports: EffectRouter + RngHandler ──────────────────────────────

#[cfg(not(target_family = "wasm"))]
use std::sync::{Arc, Weak};

#[cfg(not(target_family = "wasm"))]
use crux_core::{
    bridge::BincodeFfiFormat,
    effects::{EffectRouter, Routes, routes::Serialized},
    macros::effect,
    render::RenderOperation,
};

#[cfg(not(target_family = "wasm"))]
use crux_http::protocol::HttpRequest;

#[cfg(not(target_family = "wasm"))]
use crate::{rng_handler::RngHandler, sse::SseRequest};

// ── Wasm imports: middleware bridge ──────────────────────────────────────────

#[cfg(target_family = "wasm")]
use std::sync::Arc;

#[cfg(target_family = "wasm")]
use crux_core::middleware::{BincodeFfiFormat, Bridge, Layer as _};

// FFI Effect enum – used only by the non-wasm routing path.
// Uses #[effect(facet_typegen)] to get the EffectFfi companion type and the
// EffectFFI trait impl required by Serialized::serialize. The Export impl that
// the macro also generates is harmless: register_app::<Counter>() walks from
// app::Effect, never from ffi::Effect, so nothing here lands in typegen output.
#[cfg(not(target_family = "wasm"))]
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
}

#[cfg(not(target_family = "wasm"))]
#[allow(clippy::fallible_impl_from)] // panic on Random is intentional: it is never routed here
impl From<crate::Effect> for Effect {
    fn from(effect: crate::Effect) -> Self {
        match effect {
            crate::Effect::Render(request) => Self::Render(request),
            crate::Effect::Http(request) => Self::Http(request),
            crate::Effect::ServerSentEvents(request) => Self::ServerSentEvents(request),
            crate::Effect::Random(_) => panic!("Encountered a Random effect"),
        }
    }
}

/// For the Shell to provide.
///
/// `boltffi`'s binding generator parses the source and does not evaluate
/// `#[cfg]`, so the FFI surface (this trait and the `CoreFFI` methods below)
/// must present a single, cfg-independent signature. The native and wasm paths
/// therefore differ only inside method bodies, not in their signatures.
#[boltffi::export]
pub trait CruxShell: Send + Sync {
    /// Called when any effects resulting from an asynchronous process
    /// need processing by the shell.
    ///
    /// The bytes are a serialized vector of requests.
    fn process_effects(&self, bytes: Vec<u8>);
}

// ── Non-wasm: EffectRouter + RngHandler ──────────────────────────────────────
//
// Random is handled entirely inside Rust; the shell never sees it. Effects are
// delivered to the shell asynchronously via `process_effects`, so `update` and
// `resolve` have no synchronous effects to return.

#[cfg(not(target_family = "wasm"))]
#[derive(Clone)]
struct EffectRoutes {
    serialized: Arc<Serialized<Counter, Self, BincodeFfiFormat>>,
    rng_handler: Arc<RngHandler>,
}

#[cfg(not(target_family = "wasm"))]
impl Routes<Counter> for EffectRoutes {
    fn new(router: Weak<EffectRouter<Counter, Self>>) -> Self {
        Self {
            serialized: Arc::new(Serialized::new(router.clone())),
            rng_handler: Arc::new(RngHandler::new(router)),
        }
    }
}

/// The main interface used by the shell.
///
/// On native targets this drives an [`EffectRouter`] that handles `Random`
/// internally; on wasm (where threads are unavailable) it drives a plain
/// [`Bridge`] and `Random` is exposed to the shell.
pub struct CoreFFI {
    #[cfg(not(target_family = "wasm"))]
    router: Arc<EffectRouter<Counter, EffectRoutes>>,
    #[cfg(target_family = "wasm")]
    inner: Bridge<Core<Counter>, BincodeFfiFormat>,
}

#[boltffi::export]
#[allow(clippy::missing_panics_doc, clippy::needless_pass_by_value)]
impl CoreFFI {
    pub fn new(shell: Arc<dyn CruxShell>) -> Self {
        #[cfg(not(target_family = "wasm"))]
        {
            let router = EffectRouter::new(Core::new(), move |routes: EffectRoutes| {
                let shell = shell.clone();

                move |effect| match effect {
                    crate::Effect::Random(req) => {
                        routes.rng_handler.process(req);
                    }
                    effect => {
                        let serialized_effect = Effect::from(effect);

                        let bytes = routes
                            .serialized
                            .serialize(serialized_effect)
                            .expect("serialized effect request should encode");

                        shell.process_effects(bytes);
                    }
                }
            });

            Self { router }
        }

        #[cfg(target_family = "wasm")]
        {
            let inner = Core::<Counter>::new().bridge::<BincodeFfiFormat>(move |effect_bytes| {
                match effect_bytes {
                    Ok(effect) => shell.process_effects(effect),
                    Err(e) => panic!("{e}"),
                }
            });

            Self { inner }
        }
    }

    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.router
                .routes
                .serialized
                .update(data)
                .expect("event should deserialize");

            Vec::new()
        }

        #[cfg(target_family = "wasm")]
        {
            let mut effects = Vec::new();
            match self.inner.update(data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }
    }

    #[must_use]
    pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.router
                .routes
                .serialized
                .resolve(EffectId(effect_id), data)
                .expect("failed to resolve effect");

            Vec::new()
        }

        #[cfg(target_family = "wasm")]
        {
            let mut effects = Vec::new();
            match self.inner.resolve(EffectId(effect_id), data, &mut effects) {
                Ok(()) => effects,
                Err(e) => panic!("{e}"),
            }
        }
    }

    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        #[cfg(not(target_family = "wasm"))]
        {
            self.router
                .routes
                .serialized
                .view()
                .expect("view model should serialize")
        }

        #[cfg(target_family = "wasm")]
        {
            let mut view_model = Vec::new();
            match self.inner.view(&mut view_model) {
                Ok(()) => view_model,
                Err(e) => panic!("{e}"),
            }
        }
    }
}
