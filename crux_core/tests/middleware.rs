mod app {
    use crux_core::{capability::Operation, macros::effect, render::render, App, Command};
    use crux_http::command::Http;
    use serde::{Deserialize, Serialize};

    // Inline minimal random number capability

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct RandomNumberRequest(pub Vec<usize>); // request multiple random numbers from 1 to N

    #[derive(Debug, PartialEq, Eq, Deserialize)]
    pub struct RandomNumber(pub usize);

    impl Operation for RandomNumberRequest {
        type Output = RandomNumber;
    }

    #[effect]
    #[derive(Debug)]
    pub enum Effect {
        Random(RandomNumberRequest),
        Http(crux_http::protocol::HttpRequest),
        Render(crux_core::render::RenderOperation),
    }

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
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
        type Effect = Effect;

        fn update(
            &self,
            event: Self::Event,
            model: &mut Self::Model,
        ) -> crux_core::Command<Self::Effect, Self::Event> {
            match event {
                Event::Roll(dice) => {
                    println!("Roll dice: {dice:?}");
                    if dice.is_empty() {
                        return Command::done();
                    }

                    let first_dice = dice[0];

                    let dice = dice.into_iter().map(|dice| (dice, None)).collect();
                    model.roll = Roll::InProgress(dice);

                    println!("Requesting first dice: {first_dice:?}");
                    Command::request_from_shell(RandomNumberRequest(vec![first_dice]))
                        .then_send(Event::Random)
                }
                Event::Random(new_number) => {
                    // Update the state
                    match &mut model.roll {
                        Roll::InProgress(results) => {
                            // find position of first None, insert the arriving random_number
                            // request a new random number up to the next dice
                            println!(
                                "Got random number: {new_number:?}, results so far: {results:?}",
                            );

                            let mut incomplete_dice =
                                results.iter_mut().skip_while(|(_, v)| v.is_some());

                            if let Some((_, value)) = incomplete_dice.next() {
                                // If there is an incomplete die, fill it
                                *value = Some(new_number.0);
                            }

                            // If there are no more unresolved dice, we're done
                            //
                            // NOTE: this logic is somewhat faulty since we're now requesting _two_ dice
                            // in the next step. When the first of the two arrives, we will request
                            // the second of them _again_ despite already having requested it before
                            if incomplete_dice.next().is_none() {
                                let values = results.iter().map(|(_, v)| v.unwrap()).collect();

                                println!("Roll complete");
                                model.roll = Roll::Complete(values);
                            }
                        }
                        // We solve the above problem by terminating early if the roll is already complete
                        Roll::Complete(_) | Roll::NotStarted => return Command::done(),
                    }

                    match &model.roll {
                        Roll::NotStarted => Command::done(),
                        Roll::InProgress(items) => {
                            let next_die_sizes = items
                                .iter()
                                .copied()
                                .filter_map(
                                    |(size, value)| {
                                        if value.is_none() {
                                            Some(size)
                                        } else {
                                            None
                                        }
                                    },
                                )
                                .take(2)
                                .collect();

                            println!("Requesting next dice: {next_die_sizes:?}");
                            Command::stream_from_shell(RandomNumberRequest(next_die_sizes))
                                .then_send(Event::Random)
                        }
                        Roll::Complete(items) => {
                            println!("Roll complete, submitting to API");

                            Http::post("http://dice-api.example.com/publish")
                                .body_json(&items)
                                .expect("numbers should serialize")
                                .expect_json()
                                .build()
                                .then_send(Event::Http)
                        }
                    }
                }
                Event::Http(_http_result) => {
                    model.roll = Roll::NotStarted;
                    println!("Got http result! Rendering");

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
    use std::thread::spawn;

    use crossbeam_channel::Receiver;
    use crux_core::middleware::{EffectMiddleware, EffectResolver};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    use crate::app::{RandomNumber, RandomNumberRequest};

    // Random number generating middleware
    pub struct RngMiddleware {
        jobs_tx: crossbeam_channel::Sender<(RandomNumberRequest, EffectResolver<RandomNumber>)>,
    }

    impl RngMiddleware {
        pub fn new() -> Self {
            let (jobs_tx, jobs_rx) = crossbeam_channel::unbounded::<(
                RandomNumberRequest,
                EffectResolver<RandomNumber>,
            )>();

            // Persistent background worker
            spawn(move || {
                eprintln!("Worker thread starting...");

                while let Ok((input, mut resolver)) = jobs_rx.recv() {
                    // This is a terrible RNG which always returns the highest number
                    // the die can roll
                    eprintln!("Processing job for a dice: {:?}", input.0);

                    // Send a random number n times - this simulates subscriptions
                    let count = input.0.len();
                    for (i, n) in (input.0).iter().enumerate() {
                        eprintln!("- Resolving random number #{i} of {count}",);
                        resolver.resolve(RandomNumber(*n));
                    }
                }

                eprintln!("Worker thread terminating");
            });

            Self { jobs_tx }
        }
    }

    impl EffectMiddleware for RngMiddleware {
        type Op = RandomNumberRequest;

        fn process_effect(
            &self,
            operation: RandomNumberRequest,
            resolver: EffectResolver<RandomNumber>,
        ) {
            self.jobs_tx
                .send((operation, resolver))
                .expect("Job failed to send to worker thread");
        }
    }

    pub struct FakeHttpMiddleware;

    impl EffectMiddleware for FakeHttpMiddleware {
        type Op = HttpRequest;

        fn process_effect(
            &self,
            _operation: HttpRequest,
            mut resolver: EffectResolver<HttpResult>,
        ) {
            // One-off worker
            eprintln!("Starting HTTP thread...");
            spawn(move || {
                let response = HttpResult::Ok(HttpResponse::status(201).build());

                eprintln!("Resolving HTTP request");
                resolver.resolve(response);

                eprintln!("HTTP thread terminating...");
            });
        }
    }

    pub struct RemoteTriggerHttp {
        remote: Receiver<()>,
    }

    impl RemoteTriggerHttp {
        pub fn new(remote: Receiver<()>) -> Self {
            Self { remote }
        }
    }

    impl EffectMiddleware for RemoteTriggerHttp {
        type Op = HttpRequest;

        fn process_effect(
            &self,
            _operation: HttpRequest,
            mut resolver: EffectResolver<HttpResult>,
        ) {
            let remote = self.remote.clone();

            // One-off worker
            eprintln!("Starting remote triggered HTTP thread...");
            spawn(move || {
                let response = HttpResult::Ok(HttpResponse::status(201).build());

                eprintln!("HTTP thread awaiting remote trigger");

                if let Ok(()) = remote.recv() {
                    eprintln!("Trigger received, resolving HTTP request");

                    resolver.resolve(response);
                }

                eprintln!("HTTP thread terminating...");
            });
        }
    }
}

mod tests {
    use std::{thread::sleep, time::Duration};

    use crate::{
        app::{Dice, Effect, Event},
        middleware::{FakeHttpMiddleware, RemoteTriggerHttp, RngMiddleware},
    };
    use crossbeam_channel::RecvError;
    use crux_core::bridge::JsonFfiFormat;
    use crux_core::{
        bridge::{self, Request},
        middleware::{BincodeFfiFormat, Layer as _},
        render::RenderOperation,
        Core,
    };
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};
    use crux_macros::effect;

    #[test]
    fn roll_one_dice() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects: Vec<Effect>| effects_tx.send(effects).unwrap();

        let core = Core::<Dice>::new().handle_effects_using(RngMiddleware::new());

        let effects = core.update(Event::Roll(vec![6]), effect_callback);
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

        let core = Core::<Dice>::new().handle_effects_using(RngMiddleware::new());

        let effects = core.update(Event::Roll(vec![6, 10, 20]), effect_callback);
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let http_request = effects.remove(0).into_http().unwrap().operation;

        assert_eq!(http_request.url, "http://dice-api.example.com/publish");
        assert_eq!(String::from_utf8(http_request.body).unwrap(), "[6,10,20]");
    }

    #[test]
    fn roll_three_dice_with_http() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects: Vec<Effect>| effects_tx.send(effects).unwrap();

        let inner_core: Core<Dice> = Core::new();
        let core = inner_core
            .handle_effects_using(RngMiddleware::new())
            .handle_effects_using(FakeHttpMiddleware);

        let effects = core.update(Event::Roll(vec![6, 10, 20]), effect_callback);
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let render_operation = effects.remove(0).into_render().unwrap().operation;

        assert_eq!(RenderOperation, render_operation);
    }

    #[test]
    fn roll_with_remote_trigger_http() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let (remote_tx, remote_rx) = crossbeam_channel::unbounded();

        let effect_callback = move |effects: Vec<Effect>| effects_tx.send(effects).unwrap();

        let core = Core::<Dice>::new()
            .handle_effects_using(RemoteTriggerHttp::new(remote_rx))
            .handle_effects_using(RngMiddleware::new());

        let effects = core.update(Event::Roll(vec![6]), effect_callback);
        assert!(effects.is_empty());

        // Unblock HTTP
        eprintln!("Sending remote trigger");
        let _ = remote_tx.send(());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let render_operation = effects.remove(0).into_render().unwrap().operation;

        assert_eq!(RenderOperation, render_operation);
    }

    #[test]
    fn roll_with_late_remote_trigger_http() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let (remote_tx, remote_rx) = crossbeam_channel::unbounded();

        let effect_callback = move |effects: Vec<Effect>| effects_tx.send(effects).unwrap();

        let core = Core::<Dice>::new()
            .handle_effects_using(RemoteTriggerHttp::new(remote_rx))
            .handle_effects_using(RngMiddleware::new());

        let effects = core.update(Event::Roll(vec![6]), effect_callback);
        assert!(effects.is_empty());

        // Give worker threads a chance to proceed
        sleep(Duration::from_millis(1));
        drop(core);

        // Unblock HTTP
        eprintln!("Sending remote trigger");
        let _ = remote_tx.send(());

        let Err(RecvError) = effects_rx.recv() else {
            panic!("Should not work!")
        };

        eprintln!("Test complete");
    }

    #[effect]
    pub enum NarrowEffect {
        Render(RenderOperation),
    }

    impl From<Effect> for NarrowEffect {
        fn from(effect: Effect) -> Self {
            match effect {
                Effect::Render(render_operation) => NarrowEffect::Render(render_operation),
                Effect::Random(_) => panic!("Attempted to convert Effect::Random to NarrowEffect"),
                Effect::Http(_) => panic!("Attempted to convert Effect::Http to NarrowEffect"),
            }
        }
    }

    #[test]
    fn roll_three_dice_with_http_and_effect_narrowing() {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects: Vec<NarrowEffect>| effects_tx.send(effects).unwrap();

        let inner_core: Core<Dice> = Core::new();
        let core = inner_core
            .handle_effects_using(RngMiddleware::new())
            .handle_effects_using(FakeHttpMiddleware)
            .map_effect::<NarrowEffect>();

        let effects = core.update(Event::Roll(vec![6, 10, 20]), effect_callback);
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let NarrowEffect::Render(request) = effects.remove(0);
        let render_operation = request.operation;

        assert_eq!(RenderOperation, render_operation);
    }

    #[effect(typegen)]
    pub enum BridgeEffect {
        Http(HttpRequest),
        Render(RenderOperation),
    }

    impl From<Effect> for BridgeEffect {
        fn from(effect: Effect) -> Self {
            match effect {
                Effect::Render(render_operation) => BridgeEffect::Render(render_operation),
                Effect::Http(http_request) => BridgeEffect::Http(http_request),
                Effect::Random(_) => panic!("Attempted to convert Effect::Random to NarrowEffect"),
            }
        }
    }

    #[test]
    #[ignore = "TODO: occasionally fails probably dues to effect ordering, fix this"]
    fn roll_three_dice_with_type_narrowing_and_bridge() -> anyhow::Result<()> {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects| effects_tx.send(effects).unwrap();

        let inner_core: Core<Dice> = Core::new();
        let core = inner_core
            .handle_effects_using(RngMiddleware::new())
            .map_effect::<BridgeEffect>()
            .bridge::<BincodeFfiFormat>(effect_callback);

        let event: Vec<u8> = bincode::serialize(&Event::Roll(vec![6, 10, 20]))?;

        let mut effects_bytes = Vec::new();
        core.update(&event, &mut effects_bytes)?;
        let effects: Vec<bridge::Request<BridgeEffectFfi>> = bincode::deserialize(&effects_bytes)?;

        assert!(effects.is_empty());

        let Ok(effects_bytes) = effects_rx.recv()? else {
            panic!()
        };

        let mut effects: Vec<bridge::Request<BridgeEffectFfi>> =
            bincode::deserialize(&effects_bytes)?;

        let bridge::Request {
            effect: BridgeEffectFfi::Http(_),
            id: effect_id,
        } = effects.remove(0)
        else {
            panic!("Expected a HTTP request")
        };

        let response = HttpResult::Ok(HttpResponse::status(201).build());
        let response_bytes = bincode::serialize(&response)?;

        let mut effects_bytes = Vec::new();
        core.resolve(effect_id, &response_bytes, &mut effects_bytes)?;
        let mut effects: Vec<bridge::Request<BridgeEffectFfi>> =
            bincode::deserialize(&effects_bytes)?;

        let bridge::Request {
            effect: BridgeEffectFfi::Render(render_operation),
            ..
        } = effects.remove(0)
        else {
            panic!("Expected a HTTP request")
        };

        assert_eq!(RenderOperation, render_operation);

        Ok(())
    }

    #[test]
    #[ignore = "TODO: occasionally fails probably dues to effect ordering, fix this"]
    fn roll_three_dice_with_type_narrowing_and_json_bridge() -> anyhow::Result<()> {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects| effects_tx.send(effects).unwrap();

        let inner_core: Core<Dice> = Core::new();
        let core = inner_core
            .handle_effects_using(RngMiddleware::new())
            .map_effect::<BridgeEffect>()
            .bridge::<JsonFfiFormat>(effect_callback);

        let event = serde_json::to_vec(&Event::Roll(vec![6, 10, 20]))?;

        let mut effects_bytes = Vec::new();
        core.update(&event, &mut effects_bytes)?;
        assert_eq!(str::from_utf8(&effects_bytes)?, "[]");

        let effects: Vec<bridge::Request<BridgeEffectFfi>> =
            serde_json::from_slice(&effects_bytes)?;
        assert!(effects.is_empty());

        let Ok(effects_bytes) = effects_rx.recv()? else {
            panic!()
        };

        assert_eq!(
            str::from_utf8(&effects_bytes)?,
            r#"[{"id":0,"effect":{"Http":{"method":"POST","url":"http://dice-api.example.com/publish","headers":[{"name":"content-type","value":"application/json"}],"body":[91,54,44,49,48,44,50,48,93]}}}]"#
        );

        let mut effects: Vec<bridge::Request<BridgeEffectFfi>> =
            serde_json::from_slice(&effects_bytes)?;

        eprintln!("#1 Effects length {}", effects.len());

        let bridge::Request {
            effect: BridgeEffectFfi::Http(_),
            id: effect_id,
        } = effects.remove(0)
        else {
            panic!("Expected a HTTP request")
        };

        let response = HttpResult::Ok(HttpResponse::status(201).build());
        let response_bytes = serde_json::to_vec(&response)?;

        let mut effects_bytes = Vec::new();
        core.resolve(effect_id, &response_bytes, &mut effects_bytes)?;

        let effects_de: Vec<Request<BridgeEffectFfi>> = serde_json::from_slice(&effects_bytes)?;

        eprintln!("#2 Effects length {}", effects_de.len());
        assert_eq!(effects_de.len(), 1);
        assert!(matches!(
            effects_de[0],
            Request {
                effect: BridgeEffectFfi::Render(_),
                ..
            }
        ));

        let mut effects: Vec<bridge::Request<BridgeEffectFfi>> =
            serde_json::from_slice(&effects_bytes)?;

        let bridge::Request {
            effect: BridgeEffectFfi::Render(render_operation),
            ..
        } = effects.remove(0)
        else {
            panic!("Expected a HTTP request")
        };

        assert_eq!(RenderOperation, render_operation);

        Ok(())
    }

    /// Regression test for `<https://github.com/redbadger/crux/issues/492>`
    ///
    /// Synchronous resolution is now prevented at the API level.
    /// `EffectResolver` panics if `resolve()` is called before
    /// `process_effect` returns.
    #[test]
    #[should_panic(expected = "must not call resolve() synchronously")]
    fn synchronous_middleware_panics() {
        use crux_core::{
            capability::Operation,
            macros::effect,
            middleware::{EffectMiddleware, EffectResolver},
            Command, Core,
        };
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        struct PingOperation;

        #[derive(Debug, PartialEq, Eq, Deserialize)]
        struct PingOutput;

        impl Operation for PingOperation {
            type Output = PingOutput;
        }

        #[effect]
        #[derive(Debug)]
        enum PingEffect {
            Ping(PingOperation),
            Render(crux_core::render::RenderOperation),
        }

        #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
        enum PingEvent {
            Go,
        }

        #[derive(Default)]
        struct PingApp;

        impl crux_core::App for PingApp {
            type Event = PingEvent;
            type Model = ();
            type ViewModel = ();
            type Effect = PingEffect;

            fn update(
                &self,
                event: Self::Event,
                _model: &mut Self::Model,
            ) -> Command<Self::Effect, Self::Event> {
                match event {
                    PingEvent::Go => {
                        Command::request_from_shell(PingOperation).then_send(|_| PingEvent::Go)
                    }
                }
            }

            fn view(&self, _model: &Self::Model) -> Self::ViewModel {}
        }

        /// Middleware that attempts to resolve synchronously — this should panic.
        struct SyncPingMiddleware;

        impl EffectMiddleware for SyncPingMiddleware {
            type Op = PingOperation;

            fn process_effect(
                &self,
                _operation: PingOperation,
                mut resolver: EffectResolver<PingOutput>,
            ) {
                // This synchronous call should panic
                resolver.resolve(PingOutput);
            }
        }

        let (effects_tx, _effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects: Vec<PingEffect>| effects_tx.send(effects).unwrap();

        let core = Core::<PingApp>::new().handle_effects_using(SyncPingMiddleware);

        // This will panic inside process_effect -> resolve()
        let _ = core.update(PingEvent::Go, effect_callback);
    }
}
