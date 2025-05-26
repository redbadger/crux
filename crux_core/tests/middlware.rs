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
                Event::Http(_http_result) => render(), // we don't really care about the result, really...
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
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

    use std::{sync::Arc, thread::spawn};

    use crossbeam_channel::Sender;
    use crux_core::{capability::Operation, App, Core, Request, ResolveError};

    use crate::app::{RandomNumber, RandomNumberRequest};

    pub struct RngMiddlware<A: App, F: Fn(Vec<A::Effect>)> {
        core: Arc<Core<A>>,
        jobs_tx: crossbeam_channel::Sender<(usize, Box<dyn FnOnce(usize) + Send>)>,
        effect_callback: Arc<F>,
    }

    impl<A: App + Sync + Send + 'static, F: Fn(Vec<A::Effect>) + Send + Sync + 'static>
        RngMiddlware<A, F>
    where
        A::Effect: TryInto<Request<RandomNumberRequest>, Error = A::Effect>, // this is macro derived on effects
        A::Capabilities: Send,
    {
        pub fn new(core: Core<A>, effect_callback: F) -> Self {
            let (jobs_tx, jobs_rx) =
                crossbeam_channel::unbounded::<(usize, Box<dyn FnOnce(usize) + Send>)>();

            spawn(move || {
                eprintln!("Worker thread starting...");

                while let Ok((input, callback)) = jobs_rx.recv() {
                    // This is terrible RNG which always returns the highest number the die
                    // can produce
                    eprintln!("Processing job for a dice with  max: {input}");

                    callback(input);
                }

                eprintln!("Worker thread terminating");
            });

            Self {
                core: Arc::new(core),
                jobs_tx,
                effect_callback: Arc::new(effect_callback),
            }
        }

        pub fn process_event(&self, event: A::Event) -> Vec<A::Effect> {
            let follow_up_effects = self.core.process_event(event);

            Self::process_rand_effects(
                follow_up_effects,
                &self.core,
                self.jobs_tx.clone(),
                &self.effect_callback,
            )
        }

        pub fn resolve<Op: Operation>(
            &self,
            request: &mut Request<Op>,
            result: Op::Output,
        ) -> Result<Vec<A::Effect>, ResolveError> {
            let follow_up_effects = self.core.resolve(request, result)?;
            Ok(Self::process_rand_effects(
                follow_up_effects,
                &self.core,
                self.jobs_tx.clone(),
                &self.effect_callback,
            ))
        }

        pub fn view(&self) -> A::ViewModel {
            self.core.view()
        }

        fn process_rand_effects(
            effects: Vec<A::Effect>,
            core: &Arc<Core<A>>,
            jobs_tx: Sender<(usize, Box<dyn FnOnce(usize) + Send + 'static>)>,
            effect_callback: &Arc<F>,
        ) -> Vec<A::Effect> {
            effects
                .into_iter()
                .filter_map(|effect| {
                    match effect.try_into() {
                        Ok(
                            mut rand_request @ Request {
                                operation: RandomNumberRequest(upper_bound),
                                ..
                            },
                        ) => {
                            eprintln!("Found effect the middleware can handle!");

                            let core_weak_ref = Arc::downgrade(core);
                            let effect_callback_weak_ref = Arc::downgrade(effect_callback);
                            let callback_jobs_tx = jobs_tx.clone();

                            let when_done = Box::new(move |rn| {
                                let Some(core) = core_weak_ref.upgrade() else {
                                    // do nothing the core is gone, nobody is listening
                                    return;
                                };
                                let Some(effect_callback) = effect_callback_weak_ref.upgrade()
                                else {
                                    // do nothing the callback is gone
                                    return;
                                };

                                eprintln!("Resolving effect with random number: {rn}");

                                if let Ok(follow_up_effects) =
                                    core.resolve(&mut rand_request, RandomNumber(rn))
                                {
                                    let more_effects = Self::process_rand_effects(
                                        follow_up_effects,
                                        &core,
                                        callback_jobs_tx,
                                        &effect_callback,
                                    );

                                    if !more_effects.is_empty() {
                                        eprintln!(
                                            "Passing {} follow up effects back",
                                            more_effects.len()
                                        );

                                        effect_callback(more_effects);
                                    }
                                }
                            });

                            eprintln!("Submitting job for the background thread");

                            jobs_tx
                                .send((upper_bound, when_done))
                                .expect("could not send random number job to worker thread");
                            None
                        }
                        Err(effect) => Some(effect),
                    }
                })
                .collect()
        }
    }
}

mod tests {
    #![allow(unused_imports)]

    use crate::{
        app::{Dice, Event},
        middleware::RngMiddlware,
    };
    use crux_core::Core;

    #[test]
    fn roll_one_dice() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();

        let core: RngMiddlware<Dice, _> = RngMiddlware::new(Core::new(), {
            let effects_tx = effects_tx.clone();
            move |effects| effects_tx.send(effects).unwrap()
        });

        let effects = core.process_event(Event::Roll(vec![6]));
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

        let core: RngMiddlware<Dice, _> = RngMiddlware::new(Core::new(), {
            let effects_tx = effects_tx.clone();
            move |effects| effects_tx.send(effects).unwrap()
        });

        let effects = core.process_event(Event::Roll(vec![6, 10, 20]));
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let http_request = effects.remove(0).into_http().unwrap().operation;

        assert_eq!(http_request.url, "http://dice-api.example.com/publish");
        assert_eq!(String::from_utf8(http_request.body).unwrap(), "[6,10,20]")
    }
}
