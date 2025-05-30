mod app {
    use crux_core::{capability::Operation, macros::effect, render::render, App, Command};
    use crux_http::command::Http;
    use serde::{Deserialize, Serialize};

    // Inline minimal random number capability

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RandomNumberRequest(pub usize); // request a random number from 1 to N, inclusive

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    pub struct RandomNumber(pub usize);

    impl Operation for RandomNumberRequest {
        type Output = RandomNumber;
    }

    // Random will be handled in a middleware
    // Other effects will be passed to the shell

    #[effect]
    pub enum Effect {
        Random(RandomNumberRequest),
        Http(crux_http::protocol::HttpRequest),
        Render(crux_core::render::RenderOperation),
    }

    #[derive(Serialize, PartialEq, Eq, Debug)]
    pub enum Event {
        Roll(Vec<usize>), // roll N dice with F_i faces

        #[serde(skip)]
        Random(RandomNumber),
        #[serde(skip)]
        Http(crux_http::Result<crux_http::Response<()>>),
    }

    #[derive(Default, Debug)]
    pub struct Model {
        roll: Roll,
    }

    #[derive(Default, Debug)]
    pub enum Roll {
        #[default]
        NotStarted,
        InProgress(Vec<(usize, Option<usize>)>), // Vec<(dice, value?)>
        Complete(Vec<usize>),                    // Vec<value>
    }

    #[derive(Default)]
    pub struct Dice;

    impl App for Dice {
        type Event = Event;
        type Model = Model;
        type ViewModel = ();
        type Capabilities = ();
        type Effect = Effect;

        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
            _caps: &(),
        ) -> crux_core::Command<Self::Effect, Self::Event> {
            match event {
                Event::Roll(dice) => {
                    if dice.is_empty() {
                        return Command::done();
                    }

                    let first_dice = dice[0];

                    let dice = dice.into_iter().map(|dice| (dice, None)).collect();
                    model.roll = Roll::InProgress(dice);

                    Command::request_from_shell(RandomNumberRequest(first_dice))
                        .then_send(Event::Random)
                }
                Event::Random(new_number) => {
                    // Update the state
                    match &mut model.roll {
                        Roll::InProgress(results) => {
                            // find position of first None, insert the arriving random_number
                            // request a new random number up to the next dice

                            let mut incomplete_dice =
                                results.iter_mut().skip_while(|(_, v)| v.is_some());

                            if let Some((_, value)) = incomplete_dice.next() {
                                // If there is an incomplete die, fill it
                                *value = Some(new_number.0);
                            }

                            // If there are no more unresolved dice, we're done
                            if incomplete_dice.next().is_none() {
                                let values = results.iter().map(|(_, v)| v.unwrap()).collect();

                                model.roll = Roll::Complete(values);
                            }
                        }
                        Roll::Complete(_) | Roll::NotStarted => {}
                    };

                    match &model.roll {
                        Roll::NotStarted => Command::done(),
                        Roll::InProgress(items) => {
                            let next_die_size = items
                                .iter()
                                .copied()
                                .find_map(
                                    |(size, value)| {
                                        if value.is_none() {
                                            Some(size)
                                        } else {
                                            None
                                        }
                                    },
                                )
                                .unwrap();

                            Command::request_from_shell(RandomNumberRequest(next_die_size))
                                .then_send(Event::Random)
                        }
                        Roll::Complete(items) => Http::post("http://dice-api.example.com/publish")
                            .body_json(&items)
                            .expect("numbers should serialize")
                            .expect_json()
                            .build()
                            .then_send(Event::Http),
                    }
                }
                Event::Http(_http_result) => {
                    println!("Got http result!");

                    // we don't really care about the result, really...
                    render()
                }
            }
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            todo!()
        }
    }
}

mod middleware {
    //! This implements a middleware which generates random numnbers on a background thread
    //! and calls a callback with the result on the background thread.
    //!
    //! This demonstrates both the ability for the middlware to handle follow up effects and
    //! the thread safety of the whole operation, where the Core is called on either the main
    //! thread or the random generator thread, we don't mind which.

    use std::{
        sync::Arc,
        thread::{self, spawn},
    };

    use crux_core::{capability::Operation, App, Core, Request, ResolveError};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    use crate::app::{RandomNumber, RandomNumberRequest};

    /// A layer in the middleware stack. Implemented by the Core and the different
    /// kinds of middlewares, so that they are interchangeable
    pub trait Layer: Send + Sync {
        /// Event type expected by this layer
        type Event;
        /// Effect type emitted by this layer
        type Effect;
        /// ViewModel of this layer
        type ViewModel;

        fn process_event<F>(&self, event: Self::Event, effect_callback: F) -> Vec<Self::Effect>
        where
            F: Fn(Vec<Self::Effect>) + Clone + Send + 'static;

        fn resolve<Op, F>(
            &self,
            request: &mut Request<Op>,
            output: Op::Output,
            effect_callback: F,
        ) -> Result<Vec<Self::Effect>, ResolveError>
        where
            F: Fn(Vec<Self::Effect>) + Clone + Send + 'static,
            Op: Operation;

        fn view(&self) -> Self::ViewModel;
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

        fn process_event<F: Fn(Vec<Self::Effect>) + Send + Clone + 'static>(
            &self,
            event: Self::Event,
            _effect_callback: F,
        ) -> Vec<Self::Effect> {
            self.process_event(event)
        }

        fn resolve<Op: Operation, F: Fn(Vec<Self::Effect>) + Send + Clone + 'static>(
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
    }

    /// Middleware layer able to process some of the effects. This implements the general
    /// behaviour making sure all follow-up effects are processed and routed to the right place
    /// and delegates to the generic parameter `M`, which implements the effect processing
    /// for individual effects
    pub struct EffectMiddlewareLayer<Next, EM>
    where
        Next: Layer + Sync + Send + 'static,
        Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
        EM: EffectMiddleware<Next::Effect> + Send + Sync,
    {
        inner: Arc<(Next, EM)>,
    }

    impl<Next, M> Layer for EffectMiddlewareLayer<Next, M>
    where
        // Next layer down, core being at the bottom
        Next: Layer,
        // Effect has to try_into the operation which the middleware handles
        Next::Effect: TryInto<Request<M::Op>, Error = Next::Effect>,
        // The actual middleware effect handling implementation
        M: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
    {
        type Event = Next::Event;
        type Effect = Next::Effect;
        type ViewModel = Next::ViewModel;

        fn process_event<F: Fn(Vec<Self::Effect>) + Clone + Send + 'static>(
            &self,
            event: Self::Event,
            effect_callback: F,
        ) -> Vec<Self::Effect> {
            self.process_event(event, effect_callback)
        }

        fn resolve<Op: Operation, F: Fn(Vec<Self::Effect>) + Clone + Send + 'static>(
            &self,
            request: &mut Request<Op>,
            output: Op::Output,
            effect_callback: F,
        ) -> Result<Vec<Self::Effect>, ResolveError> {
            self.resolve(request, output, effect_callback)
        }

        fn view(&self) -> Self::ViewModel {
            self.view()
        }
    }

    impl<Next, EM> EffectMiddlewareLayer<Next, EM>
    where
        Next: Layer,
        Next::Effect: TryInto<Request<EM::Op>, Error = Next::Effect>,
        EM: EffectMiddleware<Next::Effect> + Send + Sync + 'static,
    {
        pub fn new(next: Next, middleware: EM) -> Self {
            Self {
                inner: Arc::new((next, middleware)), // TODO: make the tuple a struct
            }
        }

        // Middleware can override
        pub fn process_event(
            &self,
            event: Next::Event,
            return_effects: impl Fn(Vec<Next::Effect>) + Send + Clone + 'static,
        ) -> Vec<Next::Effect> {
            let inner = self.inner.clone();
            let return_effects_copy = return_effects.clone();

            let effects = self
                .inner
                .0
                .process_event(event, move |later_effects_from_next| {
                    // Eventual route
                    let unknown_effects = Self::process_known_effects(
                        inner.clone(),
                        later_effects_from_next,
                        return_effects.clone(),
                    );

                    if !unknown_effects.is_empty() {
                        eprintln!(
                            "Passing {} eventual effects from process_event back up",
                            unknown_effects.len()
                        );

                        return_effects(unknown_effects)
                    }
                });

            // Immediate route
            Self::process_known_effects(self.inner.clone(), effects, return_effects_copy)
        }

        // Middleware can override
        #[allow(unused)]
        pub fn resolve<Op: Operation>(
            &self,
            request: &mut Request<Op>,
            result: Op::Output,
            return_effects: impl Fn(Vec<Next::Effect>) + Clone + Send + 'static,
        ) -> Result<Vec<Next::Effect>, ResolveError> {
            let inner = self.inner.clone();
            let return_effects_copy = return_effects.clone();

            let effects =
                self.inner
                    .0
                    .resolve(request, result, move |later_effects_from_next| {
                        // Eventual route
                        let unknown_effects = Self::process_known_effects(
                            inner.clone(),
                            later_effects_from_next,
                            return_effects.clone(),
                        );

                        if !unknown_effects.is_empty() {
                            eprintln!(
                                "Passing {} eventual effects from resolve back up",
                                unknown_effects.len()
                            );

                            return_effects(unknown_effects)
                        }
                    })?;

            // Immediate route
            Ok(Self::process_known_effects(
                self.inner.clone(),
                effects,
                return_effects_copy,
            ))
        }

        // Middleware can override
        #[allow(unused)]
        pub fn view(&self) -> Next::ViewModel {
            self.inner.0.view()
        }

        fn process_known_effects(
            inner: Arc<(Next, EM)>,
            effects: Vec<Next::Effect>,
            // callback to pass effects back to previous
            return_effects: impl Fn(Vec<Next::Effect>) + Send + Clone + 'static,
        ) -> Vec<Next::Effect> {
            effects
                .into_iter()
                .filter_map(|effect| {
                    let future_inner = Arc::downgrade(&inner);

                    // Ask middleware impl to process the effect
                    // calling back with the result, potentially on a different thread (!)
                    let result = inner.1.try_process_effect_with(
                        effect,
                        // "resolve_effect"
                        {
                            let return_effects = return_effects.clone();
                            move |mut effect_request, effect_out_value| {
                                // --- thread boundary ---

                                // this is likely to be called from another thread, which did the
                                // actual effect processing work. That means this closure needs
                                // to be Send, and so does everything captured in it

                                // This allows us to do the recursion without requiring `inner` to outlive 'static
                                let Some(inner) = future_inner.upgrade() else {
                                    // do nothing, self is gone, we can't process further effects
                                    return;
                                };

                                if let Ok(immediate_effects) =
                                    inner.0.resolve(&mut effect_request, effect_out_value, {
                                        let return_effects = return_effects.clone();

                                        // Eventual eventual route
                                        move |eventual_effects| {
                                            let Some(inner) = future_inner.upgrade() else {
                                                // do nothing, self is gone, we can't process further effects
                                                return;
                                            };

                                            // Process known effects
                                            let unknown_effects = Self::process_known_effects(
                                                inner,
                                                eventual_effects,
                                                // copy of return_effects
                                                // to be used for any follow-up effects after the known ones
                                                return_effects.clone(),
                                            );

                                            // and return unknown ones
                                            if !unknown_effects.is_empty() {
                                                eprintln!(
                                                    "Passing {} eventual effects from middleware resolve back up",
                                                    unknown_effects.len()
                                                );

                                                return_effects(unknown_effects)
                                            }
                                        }
                                    })
                                {
                                    // Eventual immediate route
                                    let more_effects = Self::process_known_effects(
                                        inner,
                                        immediate_effects,
                                        return_effects.clone(),
                                    );

                                    if !more_effects.is_empty() {
                                        eprintln!(
                                            "Passing {} immediate effects from middleware resolve back up",
                                            more_effects.len()
                                        );

                                        return_effects(more_effects);
                                    }
                                }
                            }
                        },
                    );

                    Self::into_option(result)
                })
                .collect()
        }

        #[inline]
        fn into_option(result: Result<(), Next::Effect>) -> Option<Next::Effect> {
            match result {
                Ok(()) => None,
                Err(effect) => Some(effect),
            }
        }
    }

    // Crux owned Effect Middleware trait
    pub trait EffectMiddleware<Effect>
    where
        Effect: TryInto<Request<Self::Op>, Error = Effect>,
    {
        type Op: Operation;

        fn try_process_effect_with(
            &self,
            effect: Effect,
            resolve_callback: impl FnOnce(Request<Self::Op>, <Self::Op as Operation>::Output)
                + Send
                + 'static,
        ) -> Result<(), Effect>;
    }

    // Random number generating middleware
    pub struct RngMiddleware {
        jobs_tx:
            crossbeam_channel::Sender<(RandomNumberRequest, Box<dyn FnOnce(RandomNumber) + Send>)>,
    }

    impl RngMiddleware {
        pub fn new() -> Self {
            let (jobs_tx, jobs_rx) = crossbeam_channel::unbounded::<(
                RandomNumberRequest,
                Box<dyn FnOnce(RandomNumber) + Send>,
            )>();

            spawn(move || {
                eprintln!("Worker thread starting...");

                while let Ok((input, callback)) = jobs_rx.recv() {
                    // This is terrible RNG which always returns the highest number the die
                    // can produce
                    eprintln!("Processing job for a dice with  max: {}", input.0);

                    callback(RandomNumber(input.0));
                }

                eprintln!("Worker thread terminating");
            });

            Self { jobs_tx }
        }
    }

    impl<Effect> EffectMiddleware<Effect> for RngMiddleware
    where
        Effect: TryInto<Request<RandomNumberRequest>, Error = Effect>,
    {
        type Op = RandomNumberRequest;

        fn try_process_effect_with(
            &self,
            effect: Effect,
            resolve_callback: impl FnOnce(Request<RandomNumberRequest>, RandomNumber) + Send + 'static,
        ) -> Result<(), Effect> {
            let rand_request @ Request {
                operation: RandomNumberRequest(_),
                ..
            } = effect.try_into()?;

            self.jobs_tx
                .send((
                    rand_request.operation.clone(),
                    Box::new(move |number| resolve_callback(rand_request, number)),
                ))
                .expect("Job failed to send to worker thread");

            Ok(())
        }
    }

    pub struct FakeHttpMiddleware;

    impl<Effect> EffectMiddleware<Effect> for FakeHttpMiddleware
    where
        Effect: TryInto<Request<HttpRequest>, Error = Effect>,
    {
        type Op = HttpRequest;

        fn try_process_effect_with(
            &self,
            effect: Effect,
            resolve_callback: impl FnOnce(Request<Self::Op>, <Self::Op as Operation>::Output)
                + Send
                + 'static,
        ) -> Result<(), Effect> {
            let http_request @ Request {
                operation: HttpRequest { .. },
                ..
            } = effect.try_into()?;

            // One-off worker
            eprintln!("Starting HTTP thread...");
            spawn(move || {
                let response = HttpResult::Ok(HttpResponse::status(201).build());

                eprintln!("Resolving HTTP request");
                resolve_callback(http_request, response);

                eprintln!("HTTP thread terminating...");
            });

            Ok(())
        }
    }
}

mod tests {
    #![allow(unused_imports)]

    use std::sync::Arc;

    use crate::{
        app::{Dice, Effect, Event},
        middleware::{EffectMiddlewareLayer, FakeHttpMiddleware, RngMiddleware},
    };
    use crux_core::{render::RenderOperation, Core};

    #[test]
    fn roll_one_dice() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects: Vec<Effect>| effects_tx.send(effects).unwrap();

        let core: EffectMiddlewareLayer<Core<Dice>, RngMiddleware> =
            EffectMiddlewareLayer::new(Core::new(), RngMiddleware::new());

        let effects = core.process_event(Event::Roll(vec![6]), effect_callback);
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let http_request = effects.remove(0).into_http().unwrap().operation;

        assert_eq!(http_request.url, "http://dice-api.example.com/publish");
        assert_eq!(String::from_utf8(http_request.body).unwrap(), "[6]");
    }

    #[test]
    fn roll_three_dice() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects: Vec<Effect>| effects_tx.send(effects).unwrap();

        let core: EffectMiddlewareLayer<Core<Dice>, RngMiddleware> =
            EffectMiddlewareLayer::new(Core::new(), RngMiddleware::new());

        let effects = core.process_event(Event::Roll(vec![6, 10, 20]), effect_callback);
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let http_request = effects.remove(0).into_http().unwrap().operation;

        assert_eq!(http_request.url, "http://dice-api.example.com/publish");
        assert_eq!(String::from_utf8(http_request.body).unwrap(), "[6,10,20]")
    }

    #[test]
    fn roll_three_dice_with_http() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects: Vec<Effect>| effects_tx.send(effects).unwrap();

        let core: EffectMiddlewareLayer<
            EffectMiddlewareLayer<Core<Dice>, RngMiddleware>,
            FakeHttpMiddleware,
        > = EffectMiddlewareLayer::new(
            EffectMiddlewareLayer::new(Core::new(), RngMiddleware::new()),
            FakeHttpMiddleware,
        );

        let effects = core.process_event(Event::Roll(vec![6, 10, 20]), effect_callback);
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let render_operation = effects.remove(0).into_render().unwrap().operation;

        assert_eq!(RenderOperation, render_operation);
    }
}
