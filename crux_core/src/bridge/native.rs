use std::sync::Arc;

use super::native_registry::NativeResolveRegistry;
use super::{EffectId, NativeBridgeError};
use crate::core::EffectNative;
use crate::middleware::Layer;

/// Callback type for delivering native typed effects to the shell.
///
/// The callback receives `(EffectId, Ffi)` — the app-level code wraps
/// these into a concrete `NativeRequest` struct before forwarding to
/// the platform shell.
pub type NativeEffectCallback<Ffi> = dyn Fn(EffectId, Ffi) + Send + Sync + 'static;

/// Native typed bridge wrapping a middleware `Layer`.
///
/// Unlike the byte-serializing [`Bridge`](super::Bridge), `NativeBridge` delivers
/// effects as typed values via a callback closure. Both immediate effects
/// (returned synchronously from `Layer::update`) and async effects (produced
/// by middleware on background threads) flow through the same callback.
///
/// # Usage
///
/// ```rust,ignore
/// // Without middleware
/// Core::<App>::new()
///     .native_bridge(move |id, effect| {
///         shell.handle_effect(NativeRequest { id: id.0, effect });
///     })
///
/// // With middleware
/// Core::<App>::new()
///     .handle_effects_using(LiveKitMiddleware::new())
///     .native_bridge(move |id, effect| {
///         shell.handle_effect(NativeRequest { id: id.0, effect });
///     })
/// ```
pub struct NativeBridge<Next>
where
    Next: Layer,
    Next::Effect: EffectNative,
{
    next: Next,
    effect_callback: Arc<NativeEffectCallback<<Next::Effect as EffectNative>::Ffi>>,
    registry: Arc<NativeResolveRegistry<<Next::Effect as EffectNative>::Output>>,
}

impl<Next> NativeBridge<Next>
where
    Next: Layer,
    Next::Effect: EffectNative,
{
    /// Create a new `NativeBridge` wrapping the given `Layer` with a callback
    /// for delivering effects to the shell.
    pub fn new<F>(next: Next, effect_callback: F) -> Self
    where
        F: Fn(EffectId, <Next::Effect as EffectNative>::Ffi) + Send + Sync + 'static,
    {
        Self {
            next,
            effect_callback: Arc::new(effect_callback),
            registry: Arc::new(NativeResolveRegistry::new()),
        }
    }

    /// Send a typed event to the core.
    ///
    /// Effects are delivered to the shell via the callback provided at construction.
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok(())`. The error type is reserved for
    /// future use with middleware that may fail during event processing.
    pub fn update(&self, event: Next::Event) -> Result<(), NativeBridgeError> {
        let async_cb = self.make_async_callback();
        let effects = self.next.update(event, async_cb);
        self.push_effects(effects);
        Ok(())
    }

    /// Resolve a previously requested effect with a typed output.
    ///
    /// The `id` must match the `EffectId` from the original effect delivery.
    /// Follow-up effects are delivered to the shell via the callback.
    ///
    /// # Errors
    ///
    /// Returns `NativeBridgeError::ProcessResponse` if the effect id is not found,
    /// `NativeBridgeError::OutputMismatch` if the output variant doesn't match
    /// the expected type, or `NativeBridgeError::ResolveFinished` if a streaming
    /// resolve has concluded.
    pub fn resolve(
        &self,
        id: EffectId,
        output: <Next::Effect as EffectNative>::Output,
    ) -> Result<(), NativeBridgeError> {
        self.registry.resume(id, output)?;

        let async_cb = self.make_async_callback();
        let effects = self.next.process_tasks(async_cb);
        self.push_effects(effects);
        Ok(())
    }

    /// Get the current view model.
    pub fn view(&self) -> Next::ViewModel {
        self.next.view()
    }

    /// Create a callback closure for the `Layer`'s `effect_callback` parameter.
    ///
    /// This handles effects produced asynchronously by middleware (LiveKit, etc.)
    /// on background threads.
    ///
    /// # Thread safety
    ///
    /// The closure captures `Arc<Registry>` and `Arc<Callback>`:
    /// - Registry uses `Mutex` internally — safe for concurrent access
    /// - Callback must be thread-safe — enforced by `Send + Sync` bounds
    /// - Middleware may call this closure from any thread at any time
    fn make_async_callback(&self) -> impl Fn(Vec<Next::Effect>) + Send + Sync + 'static {
        let callback = self.effect_callback.clone();
        let registry = self.registry.clone();

        move |effects: Vec<Next::Effect>| {
            for eff in effects {
                let (ffi, resolve) = eff.into_native();
                let id = registry.insert(resolve);
                callback(id, ffi);
            }
        }
    }

    /// Push immediate effects to the shell via callback.
    fn push_effects(&self, effects: Vec<Next::Effect>) {
        for eff in effects {
            let (ffi, resolve) = eff.into_native();
            let id = self.registry.insert(resolve);
            (self.effect_callback)(id, ffi);
        }
    }
}
