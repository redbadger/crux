//! Middlware which can be wrapped around the Core to modify its behaviour.
//!
//! This is useful for changing the mechanics of the Core without modifying the actual
//! behaviour of the app.
//!
//! Currently, the main use-case is processing effects requested by the app inside the core,
//! but outside the app itself (which is side-effect free and synchronous). To do this,
//! use [`Layer::handle_effects_using`] and provide an implementation of [`EffectMiddleware`].
//!
//! Note that apps using middleware must be `Send` and `Sync`, because the effect middlwares
//! are expected to process effects on a separate thread (in order not to block the thread
//! the core was originally called on), and resolve them on that thread - different from the
//! original thread where they were requested. See [`EffectMiddleware`] for more discussion.
//!
//! Note: In the documentation we refer to the directions in the middleware chain
//! as "down" - towards the core, and "up" - away from the Core, towards the Shell.
use crate::{bridge::BridgeError, capability::Operation, App, Core, Effect, Request, ResolveError};

mod bridge;
mod effect_conversion;
mod effect_handling;
mod formats;

pub use bridge::{Bridge, FfiFormat};
pub use effect_conversion::MapEffectLayer;
pub use effect_handling::{EffectMiddleware, HandleEffectLayer};
pub use formats::BincodeFfiFormat;
use serde::Deserialize;

/// A layer in the middleware stack.
///
/// This is implemented by the Core and the different types of middlewares,
/// so that they can be composed as requied.
///
/// This is the lower-level of the middleware traits. You might want to implement this
/// for middlware which filters or transforms events or your view model, with awareness
/// of your app's Event and ViewModel types.
///
/// If you want to build a reusable effect-handling middleware, see [`EffectMiddleware`].
pub trait Layer: Send + Sync + Sized {
    /// Event type expected by this layer
    type Event;
    /// Effect type returned by this layer
    type Effect;
    /// ViewModel returned by this layer
    type ViewModel;

    /// Process event from the Shell. Compared to [`Core::process_event`] this expects an
    /// additional argument - a callback to be called with effects requested outside of the
    /// initial call context.
    ///
    /// The callback is used in scenarios where an effect handling middleware has handled and
    /// resolved an effect, and received follow-up effects (from the next layer down), which
    /// it cannot process. This may happen some time after the initial `process_event`
    /// call from the shell, and on a different thread.
    ///
    /// The expected behaviour of the callback is to process the effects like a shell would
    /// and call [`Layer::resolve`] with the output of the processing.
    fn process_event<F>(&self, event: Self::Event, effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static;

    /// Resolve a requested effect. Compared to [`Core::process_event`] this expects an
    /// additional argument - a callback to be called with effects requested outside of the
    /// initial call context.
    ///
    /// The callback is used in scenarios where an effect handling middleware has handled and
    /// resolved an effect, and received follow-up effects (from the next layer down), which
    /// it cannot process. This may happen some time after this `resolve` call, and on a different
    /// thread.
    ///
    /// The expected behaviour of the callback is to process the effects like a shell would
    /// and call [`Layer::resolve`] with the output of the processing.
    fn resolve<Op, F>(
        &self,
        request: &mut Request<Op>,
        output: Op::Output,
        effect_callback: F,
    ) -> Result<Vec<Self::Effect>, ResolveError>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static,
        Op: Operation;

    /// Process any tasks in the effect runtime of the Core, which are able to proceed.
    /// The tasks may produce effects which will be returned by the core and may be
    /// processed by lower middleware layers.
    ///
    /// This is used by the [`Bridge`], when resolving effects over FFI. It can't call
    /// [`resolve`], because the `Op` type argument is not known due to the type erasure
    /// involved in serializing effects and storing request handles for the FFI.
    fn process_tasks<F>(&self, effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static;

    /// Return the current state of the view model
    fn view(&self) -> Self::ViewModel;

    /// Wrap this layer with an effect handling middleware. The `middleware` argument
    /// must implement the [`EffectMiddleware`] trait.
    fn handle_effects_using<EM>(self, middleware: EM) -> HandleEffectLayer<Self, EM>
    where
        EM: EffectMiddleware<Self::Effect> + Send + Sync + 'static,
        Self::Effect: TryInto<Request<EM::Op>, Error = Self::Effect>,
    {
        HandleEffectLayer::new(self, middleware)
    }

    /// Wrap this layer with an effect mapping middleware to change the
    /// Effect type returned.
    ///
    /// This is generally used after a number of effect handling layers to "narrow"
    /// the effect type - eliminate the variants which will never be encountered, so that
    /// exhaustive matches don't require unused branches.
    fn map_effect<NewEffect>(self) -> MapEffectLayer<Self, NewEffect>
    where
        NewEffect: From<Self::Effect> + Send + 'static,
    {
        MapEffectLayer::new(self)
    }

    fn bridge<Format: FfiFormat>(
        self,
        effect_callback: impl Fn(Result<Vec<u8>, BridgeError>) + Send + Sync + 'static,
    ) -> Bridge<Self, Format>
    where
        Self::Effect: Effect,
        Self::Event: for<'a> Deserialize<'a>,
        for<'de, 'b> &'de mut Format::Deserializer<'b>: serde::Deserializer<'b>,
        for<'se, 'b> &'se mut Format::Serializer<'b>: serde::Serializer,
    {
        Bridge::new(self, effect_callback)
    }
}

// Core is a valid Layer, but only for thread-safe Apps, because
// middlewares need to be able to run background tasks and therefore
// be thread-safe (they may get called from different threads)
impl<A: App> Layer for Core<A>
where
    A: Send + Sync + 'static,
    A::Capabilities: Send + Sync + 'static,
    A::Model: Send + Sync + 'static,
{
    type Event = A::Event;
    type Effect = A::Effect;
    type ViewModel = A::ViewModel;

    fn process_event<F: Fn(Vec<Self::Effect>) + Send + Sync + 'static>(
        &self,
        event: Self::Event,
        _effect_callback: F,
    ) -> Vec<Self::Effect> {
        self.process_event(event)
    }

    fn resolve<Op: Operation, F: Fn(Vec<Self::Effect>) + Send + Sync + 'static>(
        &self,
        request: &mut Request<Op>,
        output: Op::Output,
        _effect_callback: F,
    ) -> Result<Vec<Self::Effect>, ResolveError> {
        self.resolve(request, output)
    }

    fn view(&self) -> Self::ViewModel {
        self.view()
    }

    fn process_tasks<F>(&self, _effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static,
    {
        self.process()
    }
}
