use std::sync::{Arc, Weak};

use crate::{Request, RequestHandle, Resolvable, ResolveError, capability::Operation};

use super::Layer;

/// An effect processing middleware.
///
/// Implement this trait to provide effect processing in Rust on the core side. The two typical uses for this are
/// 1. Reusing a Rust implementation of a capability compatible with all target platforms
/// 2. Using an existing crate which is not built with Sans-IO in mind.
///
/// There are a number of considerations for doing this:
/// - The effect processing will rely on system APIs or crates which MUST be portable to all platforms
///   the library using this middleware is going to be deployed to. This is fundamentally trading off
///   portability for reuse of the Rust implementation.
/// - The middleware MUST process the effect in a non-blocking fashion on a separate thread. This thread
///   may be one of a pool from an async runtime or a simple background worker thread - this is left to the
///   implementation to decide.
/// - Due to the multi-threaded nature of the processing, the core, and therefore the app are shared between
///   threads. The app must be `Send` and `Sync`, which also forces the `Model` type to be Send and Sync.
///   This should not be a problem - `Model` should not normally be `!Send` or `!Sync`.
pub trait EffectMiddleware<Effect>
where
    Effect: TryInto<Request<Self::Op>, Error = Effect>,
{
    /// The operation type this middleware can process
    type Op: Operation;

    /// Try to process `effect` if is of the right type (can convert in to a `Request<Self::Op>`).
    ///
    /// The implementation should return `Ok(())` if the conversion succeeds, and call the `resolve_callback`
    /// with the output later on. If the effect fails to convert, it should be returned wrapped in `Err(_)`.
    ///
    /// # Errors
    ///
    /// The expected error type is the same as the input Effect type, allowing the conversion to be attempted
    /// non-destructively.
    fn try_process_effect_with(
        &self,
        effect: Effect,
        resolve_callback: impl Fn(
            RequestHandle<<Self::Op as Operation>::Output>,
            <Self::Op as Operation>::Output,
        ) + Send
        + 'static,
    ) -> Result<(), Effect>;
}

struct EffectMiddlewareLayerInner<Next, EM>
where
    Next: Layer + Sync + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
    EM: EffectMiddleware<Next::Effect> + Send + Sync,
{
    next: Next,
    middleware: EM,
}

/// Middleware layer able to process some of the effects. This implements the general
/// behaviour making sure all follow-up effects are processed and routed to the right place
/// and delegates to the generic parameter `M`, which implements [`EffectMiddleware`].
pub struct HandleEffectLayer<Next, EM>
where
    Next: Layer + Sync + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
    EM: EffectMiddleware<Next::Effect> + Send + Sync,
{
    inner: Arc<EffectMiddlewareLayerInner<Next, EM>>,
}

impl<Next, EM> Layer for HandleEffectLayer<Next, EM>
where
    // Next layer down, core being at the bottom
    Next: Layer,
    // Effect has to try_into the operation which the middleware handles
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
    // The actual middleware effect handling implementation
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    type Event = Next::Event;
    type Effect = Next::Effect;
    type ViewModel = Next::ViewModel;

    fn update<F: Fn(Vec<Self::Effect>) + Send + Sync + 'static>(
        &self,
        event: Self::Event,
        effect_callback: F,
    ) -> Vec<Self::Effect> {
        self.update(event, effect_callback)
    }

    fn resolve<Output, F: Fn(Vec<Self::Effect>) + Send + Sync + 'static>(
        &self,
        request: &mut impl Resolvable<Output>,
        output: Output,
        effect_callback: F,
    ) -> Result<Vec<Self::Effect>, ResolveError> {
        self.resolve(request, output, effect_callback)
    }

    fn view(&self) -> Self::ViewModel {
        self.view()
    }

    fn process_tasks<F>(&self, effect_callback: F) -> Vec<Self::Effect>
    where
        F: Fn(Vec<Self::Effect>) + Sync + Send + 'static,
    {
        self.process_tasks(effect_callback)
    }
}

impl<Next, EM> HandleEffectLayer<Next, EM>
where
    Next: Layer,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    /// Typically, you would would use [`Layer::handle_effects_using`] to construct a `HandleEffectLayer` instance
    /// for a specific [`EffectMiddleware`].
    pub fn new(next: Next, middleware: EM) -> Self {
        Self {
            inner: Arc::new(EffectMiddlewareLayerInner { next, middleware }),
        }
    }

    fn update(
        &self,
        event: Next::Event,
        return_effects: impl Fn(Vec<Next::Effect>) + Send + Sync + 'static,
    ) -> Vec<Next::Effect> {
        let inner = Arc::downgrade(&self.inner);
        let return_effects = Arc::new(return_effects);
        let return_effects_copy = return_effects.clone();

        let effects = self
            .inner
            .next
            .update(event, move |later_effects_from_next| {
                // Eventual route
                Self::process_known_effects_with(&inner, later_effects_from_next, &return_effects);
            });

        // Immediate route
        Self::process_known_effects(&Arc::downgrade(&self.inner), effects, &return_effects_copy)
    }

    fn resolve<Output>(
        &self,
        request: &mut impl Resolvable<Output>,
        result: Output,
        return_effects: impl Fn(Vec<Next::Effect>) + Send + Sync + 'static,
    ) -> Result<Vec<Next::Effect>, ResolveError> {
        let inner = Arc::downgrade(&self.inner);
        let return_effects = Arc::new(return_effects);
        let return_effects_copy = return_effects.clone();

        let effects = self
            .inner
            .next
            .resolve(request, result, move |later_effects_from_next| {
                Self::process_known_effects_with(&inner, later_effects_from_next, &return_effects);
            })?;

        // Immediate route
        Ok(Self::process_known_effects(
            &Arc::downgrade(&self.inner),
            effects,
            &return_effects_copy,
        ))
    }

    fn view(&self) -> Next::ViewModel {
        self.inner.next.view()
    }

    fn process_tasks<F>(&self, return_effects: F) -> Vec<Next::Effect>
    where
        F: Fn(Vec<Next::Effect>) + Sync + Send + 'static,
    {
        let inner = Arc::downgrade(&self.inner);
        let return_effects = Arc::new(return_effects);
        let return_effects_copy = return_effects.clone();

        let effects = self
            .inner
            .next
            .process_tasks(move |later_effects_from_next| {
                // Eventual route
                Self::process_known_effects_with(&inner, later_effects_from_next, &return_effects);
            });

        // Immediate route
        Self::process_known_effects(&Arc::downgrade(&self.inner), effects, &return_effects_copy)
    }

    fn process_known_effects(
        inner: &Weak<EffectMiddlewareLayerInner<Next, EM>>,
        effects: Vec<Next::Effect>,
        return_effects: &Arc<impl Fn(Vec<Next::Effect>) + Send + Sync + 'static>,
    ) -> Vec<Next::Effect> {
        effects
            .into_iter()
            .filter_map(|effect| {
                // This is where the middleware handler will send the result of its work
                let resolve_callback = {
                    let return_effects = return_effects.clone();
                    let inner = inner.clone();

                    // Ideally, we'd want the `handle` to be an `impl Resolvable`, alas,
                    // generic closures are not a thing.
                    move |mut handle: RequestHandle<<EM::Op as Operation>::Output>,
                          effect_out_value| {
                        // This allows us to do the recursion without requiring `inner` to outlive 'static
                        let Some(strong_inner) = inner.upgrade() else {
                            // do nothing, inner is gone, we can't process further effects
                            eprintln!("Inner cant't be upgraded after resolving effect");
                            return;
                        };

                        if let Ok(immediate_effects) =
                            strong_inner.next.resolve(&mut handle, effect_out_value, {
                                let return_effects = return_effects.clone();
                                let future_inner = inner.clone();

                                // Eventual eventual route
                                move |eventual_effects| {
                                    // Process known effects
                                    Self::process_known_effects_with(
                                        &future_inner,
                                        eventual_effects,
                                        &return_effects,
                                    );
                                }
                            })
                        {
                            Self::process_known_effects_with(
                                &inner,
                                immediate_effects,
                                &return_effects,
                            );
                        }
                    } // TODO: handle/propagate resolve error?
                };

                let Some(strong_inner) = inner.upgrade() else {
                    // do nothing, inner is gone, we can't process further effects
                    eprintln!("Inner cant't be upgraded to resolve effect");
                    return Some(effect);
                };

                // Ask middleware impl to process the effect
                // calling back with the result, potentially on a different thread (!)
                strong_inner
                    .middleware
                    .try_process_effect_with(effect, resolve_callback)
                    .err()
            })
            .collect()
    }

    fn process_known_effects_with(
        inner: &Weak<EffectMiddlewareLayerInner<Next, EM>>,
        effects: Vec<<Next as Layer>::Effect>,
        return_effects: &Arc<impl Fn(Vec<<Next as Layer>::Effect>) + Send + Sync + 'static>,
    ) {
        let unknown_effects = Self::process_known_effects(inner, effects, return_effects);

        if !unknown_effects.is_empty() {
            return_effects(unknown_effects);
        }
    }
}
