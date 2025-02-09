use super::Core;

/// Implement Middleware for a type which wraps the [`Core`] and modifies its behaviour in some way.
/// This allows this new type to still be wrapped in a [`Bridge`], which is  generic over an
/// implementation of `Middleware`.
///
/// Middleware gets to adapt the incoming Events, the outgoing Effect requests, and the returned view model.
///
/// Specifically, this is useful to provide Rust-side implementations of capabilities when they exist,
/// and are easily portable for the target platforms, so that they don't have to be implemented in the
/// shell several times.
pub trait Middleware {
    type App: crate::App;

    /// Process an event. The middleware may capture or modify the incoming events. Note that these are
    /// only the events originating outside the core, any internal events will be sent back to th app
    /// by the `Core` directly
    fn process_event(
        &self,
        event: <Self::App as crate::App>::Event,
    ) -> impl Iterator<Item = <Self::App as crate::App>::Effect>;

    /// Process any unfinished effects tasks and return any resulting effect requests.
    ///
    /// Implementations are expected to call this method after resolving any effect requests to advance
    /// the effects runtime.
    ///
    /// # Discussion
    ///
    /// The trait does not provide a `resolve_effect` method, because doing so would require middleware to hold
    /// original Requests in their original form. As an example, the Bridge does not do this - instead, it converts
    /// the Request into a similar type working with Serializers instead of values. The Core technically does not
    /// require the original Request in order to proceed, the requests just needs to be resolved first, the
    /// `Core` API is more of a convenience.
    ///
    // FIXME: This will generally just forward down, is there a way to provide a default implementation...?
    fn process_effects(&self) -> impl Iterator<Item = <Self::App as crate::App>::Effect>;

    /// Return the view model from the app. This gives the middleware a chance to modify the view model
    /// on the way out
    fn view(&self) -> <Self::App as crate::App>::ViewModel;
}

impl<A: crate::App> Middleware for Core<A> {
    type App = A;

    fn process_event(
        &self,
        event: <Self::App as crate::App>::Event,
    ) -> impl Iterator<Item = <Self::App as crate::App>::Effect> {
        self.process_event(event)
    }

    fn process_effects(&self) -> impl Iterator<Item = <Self::App as crate::App>::Effect> {
        self.process()
    }

    fn view(&self) -> <Self::App as crate::App>::ViewModel {
        self.view()
    }
}
