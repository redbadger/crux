#![allow(clippy::used_underscore_items)]

use crux_core::{
    Core,
    bridge::EffectId,
    macros::effect,
    middleware::{BincodeFfiFormat, Bridge, Layer as _},
    render::RenderOperation,
};
use crux_http::protocol::HttpRequest;

use std::sync::Arc;

#[cfg(not(target_family = "wasm"))]
use crux_core::middleware::{HandleEffectLayer, MapEffectLayer};

#[cfg(not(target_family = "wasm"))]
use crate::middleware::RngMiddleware;
use crate::{Counter, RandomNumberRequest, sse::SseRequest};

// ANCHOR: ffi_effect
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest),
    Random(RandomNumberRequest),
}
// ANCHOR_END: ffi_effect

// ANCHOR: ffi_from
impl From<crate::app::Effect> for Effect {
    fn from(effect: crate::app::Effect) -> Self {
        match effect {
            crate::Effect::Render(request) => Self::Render(request),
            crate::Effect::Http(request) => Self::Http(request),
            crate::Effect::ServerSentEvents(request) => Self::ServerSentEvents(request),
            crate::Effect::Random(request) => Self::Random(request),
        }
    }
}
// ANCHOR_END: ffi_from

#[cfg(not(target_family = "wasm"))]
type CoreBridge = Bridge<
    MapEffectLayer<HandleEffectLayer<Core<Counter>, RngMiddleware>, Effect>,
    BincodeFfiFormat,
>;

#[cfg(target_family = "wasm")]
type CoreBridge = Bridge<Core<Counter>, BincodeFfiFormat>;

/// For the Shell to provide.
///
/// `boltffi`'s binding generator parses the source and does not evaluate
/// `#[cfg]`, so the FFI surface (this trait and the `CoreFFI` methods below)
/// must present a single, cfg-independent signature. The native and wasm paths
/// therefore differ only inside `new`, not in any method signature.
#[boltffi::export]
pub trait CruxShell: Send + Sync {
    /// Called when any effects resulting from an asynchronous process
    /// need processing by the shell.
    ///
    /// The bytes are a serialized vector of requests.
    fn process_effects(&self, bytes: Vec<u8>);
}

/// The main interface used by the shell.
pub struct CoreFFI {
    core: CoreBridge,
}

#[boltffi::export]
#[allow(clippy::missing_panics_doc)]
impl CoreFFI {
    // ANCHOR: ffi_new
    pub fn new(shell: Arc<dyn CruxShell>) -> Self {
        // Native: RngMiddleware handles `Random` in a background task, so its
        // effects are delivered to the shell asynchronously via the callback.
        #[cfg(not(target_family = "wasm"))]
        let core = Core::<Counter>::new()
            .handle_effects_using(RngMiddleware::new())
            .map_effect::<Effect>()
            .bridge::<BincodeFfiFormat>(move |effect_bytes| match effect_bytes {
                Ok(effect) => shell.process_effects(effect),
                Err(e) => panic!("{e}"),
            });

        // Wasm: no background tasks, so effects are returned synchronously from
        // update/resolve. The callback is wired for API parity but unused.
        #[cfg(target_family = "wasm")]
        let core =
            Core::<Counter>::new().bridge::<BincodeFfiFormat>(
                move |effect_bytes| match effect_bytes {
                    Ok(effect) => shell.process_effects(effect),
                    Err(e) => panic!("{e}"),
                },
            );

        Self { core }
    }
    // ANCHOR_END: ffi_new

    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.update(data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    #[must_use]
    pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.resolve(EffectId(effect_id), data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let mut view_model = Vec::new();
        match self.core.view(&mut view_model) {
            Ok(()) => view_model,
            Err(e) => panic!("{e}"),
        }
    }
}
