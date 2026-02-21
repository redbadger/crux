use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};

use crate::{Request, RequestHandle, Resolvable, ResolveError, capability::Operation};

use super::Layer;

/// A resolver for an effect processed by middleware.
///
/// This type encapsulates the callback that feeds the operation's output back
/// into the core. It **must** be called from an asynchronous context (e.g. a
/// spawned thread, an async task via `spawn_local`, or a channel worker) —
/// calling [`resolve`](Self::resolve) while
/// [`try_process_effect`](EffectMiddleware::try_process_effect) is still on the call
/// stack will panic.
///
/// For streaming operations ([`RequestHandle::Many`]), call `resolve` multiple
/// times until the stream is exhausted.
type ResolveFn<Output> = Box<dyn FnMut(&mut RequestHandle<Output>, Output) + Send>;

pub struct EffectResolver<Output: Send + 'static> {
    handle: RequestHandle<Output>,
    resolve_fn: ResolveFn<Output>,
    /// `true` while `try_process_effect` is executing on the call stack.
    active: Arc<AtomicBool>,
}

impl<Output: Send + 'static> EffectResolver<Output> {
    /// Resolve the effect with the given output.
    ///
    /// For one-shot effects this should be called exactly once. For streaming
    /// effects it can be called multiple times.
    ///
    /// # Panics
    ///
    /// Panics if called synchronously from within
    /// [`EffectMiddleware::try_process_effect`]. Middleware must dispatch work
    /// asynchronously (e.g. `std::thread::spawn`, `spawn_local`, or a channel)
    /// and call `resolve` from there.
    ///
    /// See <https://github.com/redbadger/crux/issues/492>
    pub fn resolve(&mut self, output: Output) {
        assert!(
            !self.active.load(Ordering::Acquire),
            "EffectMiddleware::try_process_effect must not call resolve() synchronously. \
             Dispatch work asynchronously (thread, spawn_local, channel, etc.). \
             See https://github.com/redbadger/crux/issues/492"
        );
        (self.resolve_fn)(&mut self.handle, output);
    }
}

/// An effect processing middleware.
///
/// Implement this trait to provide effect processing in Rust on the core side.
/// The two typical uses for this are:
///
/// 1. Reusing a Rust implementation of a capability compatible with all target
///    platforms.
/// 2. Using an existing crate which is not built with Sans-IO in mind.
///
/// There are a number of considerations for doing this:
///
/// - The effect processing will rely on system APIs or crates which MUST be
///   portable to all platforms the library using this middleware is going to be
///   deployed to. This is fundamentally trading off portability for reuse of the
///   Rust implementation.
/// - The middleware MUST process the effect asynchronously — it must not call
///   [`EffectResolver::resolve`] before `try_process_effect` returns. On native
///   targets this typically means spawning a thread or sending work to a
///   channel-based worker. On WASM (which has no threads) this means using
///   `spawn_local` or a similar async task primitive. Calling `resolve()`
///   synchronously inside `try_process_effect` will panic.
/// - Because the resolver may be sent to another thread (on native), the core
///   and therefore the app are shared between threads. The app must be `Send`
///   and `Sync`, which also forces the `Model` type to be `Send` and `Sync`.
///   This should not be a problem — `Model` should not normally be `!Send` or
///   `!Sync`.
///
/// # Example
///
/// ```rust,ignore
/// impl EffectMiddleware for MyMiddleware {
///     type Op = MyOperation;
///
///     fn try_process_effect(
///         &self,
///         operation: MyOperation,
///         mut resolver: EffectResolver<<MyOperation as Operation>::Output>,
///     ) {
///         std::thread::spawn(move || {
///             let output = do_work(operation);
///             resolver.resolve(output);
///         });
///     }
/// }
/// ```
pub trait EffectMiddleware: Send + Sync {
    /// The operation type this middleware can process.
    type Op: Operation;

    /// Process the given operation and resolve via the provided resolver.
    ///
    /// The framework has already extracted the operation from the effect enum.
    /// Use the [`EffectResolver`] to send the result back. The resolver **must
    /// not** be called before this method returns — dispatch the work
    /// asynchronously (e.g. `std::thread::spawn` on native, `spawn_local` on
    /// WASM, or a channel send) and call [`EffectResolver::resolve`] from
    /// there.
    fn try_process_effect(
        &self,
        operation: Self::Op,
        resolver: EffectResolver<<Self::Op as Operation>::Output>,
    );
}

struct EffectMiddlewareLayerInner<Next, EM>
where
    Next: Layer + Sync + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
    EM: EffectMiddleware,
{
    next: Next,
    middleware: EM,
}

/// Middleware layer able to process some of the effects. This implements the
/// general behaviour making sure all follow-up effects are processed and routed
/// to the right place and delegates to the generic parameter `EM`, which
/// implements [`EffectMiddleware`].
pub struct HandleEffectLayer<Next, EM>
where
    Next: Layer + Sync + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
    EM: EffectMiddleware,
{
    inner: Arc<EffectMiddlewareLayerInner<Next, EM>>,
}

impl<Next, EM> Layer for HandleEffectLayer<Next, EM>
where
    Next: Layer,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
    EM: EffectMiddleware + 'static,
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
    EM: EffectMiddleware + 'static,
{
    /// Typically, you would use [`Layer::handle_effects_using`] to construct a
    /// `HandleEffectLayer` instance for a specific [`EffectMiddleware`].
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
                // Try to convert the effect into a Request for the middleware's
                // operation type. If conversion fails, the effect is not for
                // this middleware — pass it through.
                let request: Request<EM::Op> = match effect.try_into() {
                    Ok(req) => req,
                    Err(effect) => return Some(effect),
                };

                let (operation, handle) = request.split();

                // Build the resolve function that will be called from the
                // middleware's async context (thread, spawn_local, etc.).
                let resolve_fn = {
                    let return_effects = return_effects.clone();
                    let inner = inner.clone();

                    move |req_handle: &mut RequestHandle<<EM::Op as Operation>::Output>, output| {
                        let Some(strong_inner) = inner.upgrade() else {
                            eprintln!("Inner can't be upgraded after resolving effect");
                            return;
                        };

                        if let Ok(immediate_effects) =
                            strong_inner.next.resolve(req_handle, output, {
                                let return_effects = return_effects.clone();
                                let future_inner = inner.clone();

                                move |eventual_effects| {
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
                    }
                };

                let Some(strong_inner) = inner.upgrade() else {
                    eprintln!("Inner can't be upgraded to process effect");
                    return None;
                };

                // Create the resolver with the active guard.
                let active = Arc::new(AtomicBool::new(true));
                let resolver = EffectResolver {
                    handle,
                    resolve_fn: Box::new(resolve_fn),
                    active: active.clone(),
                };

                // Call the middleware. resolve() will panic if called during
                // this scope.
                strong_inner
                    .middleware
                    .try_process_effect(operation, resolver);

                // Allow resolve() to be called now that try_process_effect has returned.
                active.store(false, Ordering::Release);

                None
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
