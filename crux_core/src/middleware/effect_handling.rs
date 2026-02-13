use std::sync::{Arc, Weak, atomic::AtomicBool};

use crossbeam_channel::{Receiver, Sender, unbounded};

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
        resolve_callback: impl FnMut(
            &mut RequestHandle<<Self::Op as Operation>::Output>,
            <Self::Op as Operation>::Output,
        ) + Send
        + 'static,
    ) -> Result<(), Effect>;
}

struct EffectMiddlewareLayerInner<Next, EM>
where
    Next: Layer + Sync + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
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
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    inner: Arc<EffectMiddlewareLayerInner<Next, EM>>,
    worker: Worker<Next, EM>,
}

impl<Next, EM> Layer for HandleEffectLayer<Next, EM>
where
    // Next layer down, core being at the bottom
    Next: Layer,
    // Effect has to try_into the operation which the middleware handles
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
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
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    /// Typically, you would would use [`Layer::handle_effects_using`] to construct a `HandleEffectLayer` instance
    /// for a specific [`EffectMiddleware`].
    pub fn new(next: Next, middleware: EM) -> Self {
        Self {
            inner: Arc::new(EffectMiddlewareLayerInner { next, middleware }),
            worker: Worker::default(),
        }
    }

    fn update(
        &self,
        event: Next::Event,
        return_effects: impl Fn(Vec<Next::Effect>) + Send + Sync + 'static,
    ) -> Vec<Next::Effect> {
        let inner = Arc::downgrade(&self.inner);
        let return_effects: Arc<dyn Fn(Vec<Next::Effect>) + Send + Sync + 'static> =
            Arc::new(return_effects);
        let return_effects_copy = return_effects.clone();

        let worker = self.worker.clone();
        let effects = self
            .inner
            .next
            .update(event, move |later_effects_from_next| {
                // Eventual route
                Self::process_known_effects_with(
                    &worker,
                    &inner,
                    later_effects_from_next,
                    &return_effects,
                );
            });

        // Immediate route
        Self::process_known_effects(
            &self.worker,
            &Arc::downgrade(&self.inner),
            effects,
            &return_effects_copy,
        )
    }

    fn resolve<Output>(
        &self,
        request: &mut impl Resolvable<Output>,
        result: Output,
        return_effects: impl Fn(Vec<Next::Effect>) + Send + Sync + 'static,
    ) -> Result<Vec<Next::Effect>, ResolveError> {
        let inner = Arc::downgrade(&self.inner);
        let return_effects: Arc<dyn Fn(Vec<Next::Effect>) + Send + Sync + 'static> =
            Arc::new(return_effects);
        let return_effects_copy = return_effects.clone();
        let worker = self.worker.clone();

        let effects = self
            .inner
            .next
            .resolve(request, result, move |later_effects_from_next| {
                Self::process_known_effects_with(
                    &worker,
                    &inner,
                    later_effects_from_next,
                    &return_effects,
                );
            })?;

        // Immediate route
        Ok(Self::process_known_effects(
            &self.worker,
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

        let return_effects: Arc<dyn Fn(Vec<Next::Effect>) + Send + Sync + 'static> =
            Arc::new(return_effects);
        let return_effects_copy = return_effects.clone();
        let worker = self.worker.clone();
        let effects = self
            .inner
            .next
            .process_tasks(move |later_effects_from_next| {
                // Eventual route
                Self::process_known_effects_with(
                    &worker,
                    &inner,
                    later_effects_from_next,
                    &return_effects,
                );
            });

        // Immediate route
        Self::process_known_effects(
            &self.worker,
            &Arc::downgrade(&self.inner),
            effects,
            &return_effects_copy,
        )
    }

    #[track_caller]
    fn process_known_effects(
        worker: &Worker<Next, EM>,
        inner: &Weak<EffectMiddlewareLayerInner<Next, EM>>,
        effects: Vec<Next::Effect>,
        return_effects: &Arc<dyn Fn(Vec<Next::Effect>) + Send + Sync + 'static>,
    ) -> Vec<Next::Effect> {
        effects
            .into_iter()
            .filter_map(|effect| {
                // This is where the middleware handler will send the result of its work
                let resolve_callback = {
                    let return_effects = return_effects.clone();
                    let inner = inner.clone();
                    let worker = worker.clone();

                    // Ideally, we'd want the `handle` to be an `impl Resolvable`, alas,
                    // generic closures are not a thing.
                    move |handle: &mut RequestHandle<<EM::Op as Operation>::Output>,
                          effect_out_value| {
                        // This allows us to do the recursion without requiring `inner` to outlive 'static
                        let Some(strong_inner) = inner.upgrade() else {
                            // do nothing, inner is gone, we can't process further effects
                            eprintln!("Inner cant't be upgraded after resolving effect");
                            return;
                        };
                        let return_effects = return_effects.clone();
                        let inner = inner.clone();

                        // We don't want to overflow the stack by recursively calling
                        // back and forth between middlewares and the core.
                        //
                        // So if we made one roundtrip, and we are asked to make more,
                        // we deem it as good a time as any to enqueue further processing.
                        // This is done via the `worker`.
                        if let Ok(immediate_effects) =
                            strong_inner.next.resolve(handle, effect_out_value, {
                                let return_effects = return_effects.clone();
                                let future_inner = inner.clone();
                                let worker = worker.clone();

                                move |eventual_effects| {
                                    let return_effects = return_effects.clone();
                                    let future_inner = future_inner.clone();
                                    let more_effects_to_resolve = MiddlewareTask(
                                        future_inner,
                                        eventual_effects,
                                        return_effects,
                                    );

                                    worker.enqueue(more_effects_to_resolve);
                                }
                            })
                        {
                            let more_effects_to_resolve =
                                MiddlewareTask(inner.clone(), immediate_effects, return_effects);
                            worker.enqueue(more_effects_to_resolve);
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
        worker: &Worker<Next, EM>,
        inner: &Weak<EffectMiddlewareLayerInner<Next, EM>>,
        effects: Vec<<Next as Layer>::Effect>,
        return_effects: &Arc<dyn Fn(Vec<Next::Effect>) + Send + Sync + 'static>,
    ) {
        let unknown_effects = Self::process_known_effects(worker, inner, effects, return_effects);

        if !unknown_effects.is_empty() {
            return_effects(unknown_effects);
        }
    }
}

struct MiddlewareTask<Next, EM, EffectCb>(
    Weak<EffectMiddlewareLayerInner<Next, EM>>,
    Vec<<Next as Layer>::Effect>,
    Arc<EffectCb>,
)
where
    Next: Layer + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
    EffectCb: Fn(Vec<Next::Effect>) + Send + Sync + ?Sized + 'static;

type SendMiddlewareTask<Next, EM> =
    MiddlewareTask<Next, EM, dyn Fn(Vec<<Next as Layer>::Effect>) + Send + Sync + 'static>;

struct Worker<Next, EM>
where
    Next: Layer + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    sender: Sender<SendMiddlewareTask<Next, EM>>,
    receiver: Receiver<SendMiddlewareTask<Next, EM>>,
    is_working: Arc<AtomicBool>,
}

impl<Next, EM> Clone for Worker<Next, EM>
where
    Next: Layer + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            is_working: self.is_working.clone(),
        }
    }
}

impl<Next, EM> Default for Worker<Next, EM>
where
    Next: Layer + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    fn default() -> Self {
        // Do we want any sort of backpressure here?
        // I would assume no because:
        // - We don't want the hot path to be stalled
        // - As soon as a task is enqueued a worker should either:
        //   - Start `pop()`ing the queue
        //   - Notice an other part of the program is already doing it
        // The channel should thus almost always be empty
        // On the other hand there is no such thing as unbounded channel
        // because machines have a finite set of resources...
        let (sender, receiver) = unbounded();
        Self {
            sender,
            receiver,
            is_working: Arc::default(),
        }
    }
}

impl<Next, EM> Worker<Next, EM>
where
    Next: Layer + Send + 'static,
    Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect> + Send,
    EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
{
    // Add a middleware task to be processed by the worker.
    // If no part of the codebase is actively processing tasks,
    // We do it ourselves.
    //
    // These steps act as a stackoverflow guard,
    // as the topmost middleware tries to pop() as much work as possible
    fn enqueue(&self, work: SendMiddlewareTask<Next, EM>) {
        self.sender.send(work).unwrap();
        self.work_if_needed();
    }

    fn work_if_needed(&self) {
        if self
            .is_working
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_ok()
        {
            // No one else is working, let's get going
            self.clone().work();

            self.is_working
                .store(false, std::sync::atomic::Ordering::SeqCst);
            // There could be a situation in which is_working is about to be set to false.
            // and someone is enqueuing work.
            // This is why we work again, just in case.
            // In the worst case scenario we start to work and yield immediately because there is nothing to do.
            // In the best case scenario we catch some more work and execute it immediately
            self.clone().work();
        }
    }

    fn work(self) {
        // No one else is doing the work, let's get to it
        while let Ok(MiddlewareTask(maybe_middleware, effects_to_process, return_effects)) =
            self.receiver.try_recv()
        {
            HandleEffectLayer::process_known_effects_with(
                &self,
                &maybe_middleware,
                effects_to_process,
                &return_effects,
            );
        }
    }
}
