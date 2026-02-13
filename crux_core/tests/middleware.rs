mod app {
    use crux_core::{App, Command, capability::Operation, macros::effect, render::render};
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
                                        if value.is_none() { Some(size) } else { None }
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
    use crux_core::{Request, RequestHandle, capability::Operation, middleware::EffectMiddleware};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    use crate::app::{RandomNumber, RandomNumberRequest};

    // Random number generating middleware
    #[allow(clippy::type_complexity)]
    pub struct RngMiddleware {
        jobs_tx:
            crossbeam_channel::Sender<(RandomNumberRequest, Box<dyn FnMut(RandomNumber) + Send>)>,
    }

    impl RngMiddleware {
        pub fn new() -> Self {
            let (jobs_tx, jobs_rx) = crossbeam_channel::unbounded::<(
                RandomNumberRequest,
                Box<dyn FnMut(RandomNumber) + Send>,
            )>();

            // Persistent background worker
            spawn(move || {
                eprintln!("Worker thread starting...");

                while let Ok((input, mut callback)) = jobs_rx.recv() {
                    // This is a terrible RNG which always returns the highest number
                    // the die can roll
                    eprintln!("Processing job for a dice: {:?}", input.0);

                    // Send a random number n times - this simulates subscriptions
                    let count = input.0.len();
                    for (i, n) in (input.0).iter().enumerate() {
                        eprintln!("- Resolving random number #{i} of {count}",);
                        callback(RandomNumber(*n));
                    }
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
            mut resolve_callback: impl FnMut(&mut RequestHandle<RandomNumber>, RandomNumber)
            + Send
            + 'static,
        ) -> Result<(), Effect> {
            let rand_request = effect.try_into()?;
            let (operation, mut handle): (RandomNumberRequest, RequestHandle<_>) =
                rand_request.split();

            let callback = move |number| resolve_callback(&mut handle, number);

            self.jobs_tx
                .send((operation, Box::new(callback)))
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
            mut resolve_callback: impl FnMut(
                &mut RequestHandle<<Self::Op as Operation>::Output>,
                <Self::Op as Operation>::Output,
            ) + Send
            + 'static,
        ) -> Result<(), Effect> {
            let http_request = effect.try_into()?;
            let (_, mut handle): (HttpRequest, _) = http_request.split();

            // One-off worker
            eprintln!("Starting HTTP thread...");
            spawn(move || {
                let response = HttpResult::Ok(HttpResponse::status(201).build());

                eprintln!("Resolving HTTP request");
                resolve_callback(&mut handle, response);

                eprintln!("HTTP thread terminating...");
            });

            Ok(())
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

    impl<Effect> EffectMiddleware<Effect> for RemoteTriggerHttp
    where
        Effect: TryInto<Request<HttpRequest>, Error = Effect>,
    {
        type Op = HttpRequest;

        fn try_process_effect_with(
            &self,
            effect: Effect,
            mut resolve_callback: impl FnMut(
                &mut RequestHandle<<Self::Op as Operation>::Output>,
                <Self::Op as Operation>::Output,
            ) + Send
            + 'static,
        ) -> Result<(), Effect> {
            let http_request = effect.try_into()?;

            let (_, mut handle): (HttpRequest, _) = http_request.split();
            let remote = self.remote.clone();

            // One-off worker
            eprintln!("Starting remote triggered HTTP thread...");
            spawn(move || {
                let response = HttpResult::Ok(HttpResponse::status(201).build());

                eprintln!("HTTP thread awaiting remote trigger");

                if let Ok(()) = remote.recv() {
                    eprintln!("Trigger received, resolving HTTP request");

                    resolve_callback(&mut handle, response);
                }

                eprintln!("HTTP thread terminating...");
            });

            Ok(())
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
        Core,
        bridge::{self, Request},
        middleware::{BincodeFfiFormat, Layer as _},
        render::RenderOperation,
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

    /// Regression test for https://github.com/redbadger/crux/issues/492
    ///
    /// When middleware resolves effects synchronously (on the same thread, inside
    /// `try_process_effect_with`), the resolve chain used to recurse through
    /// Core::process() → middleware → resolve_callback → Core::process() → ...
    /// causing a stack overflow for commands with many sequential requests.
    #[test]
    fn synchronous_middleware_doesnt_blow_the_stack() {
        use crux_core::{
            capability::Operation, macros::effect, middleware::EffectMiddleware, Command, Core,
            Request, RequestHandle,
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
                    PingEvent::Go => Command::new(|ctx| async move {
                        // Many sequential requests — previously caused stack overflow
                        for _ in 0..10_000 {
                            ctx.request_from_shell(PingOperation).await;
                        }
                    }),
                }
            }

            fn view(&self, _model: &Self::Model) -> Self::ViewModel {}
        }

        /// Middleware that resolves PingOperation **synchronously** on the same thread.
        struct SyncPingMiddleware;

        impl<Eff> EffectMiddleware<Eff> for SyncPingMiddleware
        where
            Eff: TryInto<Request<PingOperation>, Error = Eff>,
        {
            type Op = PingOperation;

            fn try_process_effect_with(
                &self,
                effect: Eff,
                mut resolve_callback: impl FnMut(&mut RequestHandle<PingOutput>, PingOutput)
                    + Send
                    + 'static,
            ) -> Result<(), Eff> {
                let request = effect.try_into()?;
                let (_operation, mut handle) = request.split();

                // Resolve immediately on the same thread — this is what triggers
                // the re-entrant call to Core::process()
                resolve_callback(&mut handle, PingOutput);

                Ok(())
            }
        }

        let (effects_tx, _effects_rx) = crossbeam_channel::unbounded();
        let effect_callback =
            move |effects: Vec<PingEffect>| effects_tx.send(effects).unwrap();

        let core = Core::<PingApp>::new()
            .handle_effects_using(SyncPingMiddleware);

        // This previously caused: "thread has overflowed its stack"
        let effects = core.update(PingEvent::Go, effect_callback);

        // All Ping effects were handled by middleware, so only unhandled
        // effects (if any) come back here. The important thing is we
        // didn't crash.
        assert!(
            effects.is_empty(),
            "All Ping effects should be handled by middleware"
        );
    }
}
