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
    use std::thread::spawn;

    use crossbeam_channel::Receiver;
    use crux_core::{capability::Operation, middleware::EffectMiddleware, Request};
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    use crate::app::{RandomNumber, RandomNumberRequest};

    // Random number generating middleware
    #[allow(clippy::type_complexity)]
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

            // Persistent background worker
            spawn(move || {
                eprintln!("Worker thread starting...");

                while let Ok((input, callback)) = jobs_rx.recv() {
                    // This is a terrible RNG which always returns the highest number
                    // the die can roll
                    eprintln!("Processing job for a die with max: {}", input.0);

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
            resolve_callback: impl FnOnce(Request<Self::Op>, <Self::Op as Operation>::Output)
                + Send
                + 'static,
        ) -> Result<(), Effect> {
            let http_request @ Request {
                operation: HttpRequest { .. },
                ..
            } = effect.try_into()?;

            let remote = self.remote.clone();

            // One-off worker
            eprintln!("Starting remote triggered HTTP thread...");
            spawn(move || {
                let response = HttpResult::Ok(HttpResponse::status(201).build());

                eprintln!("HTTP thread awaiting remote trigger");

                if let Ok(()) = remote.recv() {
                    eprintln!("Trigger received, resolving HTTP request");

                    resolve_callback(http_request, response);
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
    use bincode::{
        config::{AllowTrailing, FixintEncoding, WithOtherIntEncoding, WithOtherTrailing},
        de::read::SliceReader,
        DefaultOptions, Options as _,
    };
    use crossbeam_channel::RecvError;
    use crux_core::{
        bridge,
        middleware::{FfiFormat, Layer as _},
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

        let core = Core::<Dice>::new().handle_effects_using(RngMiddleware::new());

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

        let inner_core: Core<Dice> = Core::new();
        let core = inner_core
            .handle_effects_using(RngMiddleware::new())
            .handle_effects_using(FakeHttpMiddleware);

        let effects = core.process_event(Event::Roll(vec![6, 10, 20]), effect_callback);
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

        let effects = core.process_event(Event::Roll(vec![6]), effect_callback);
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

        let effects = core.process_event(Event::Roll(vec![6]), effect_callback);
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

        let effects = core.process_event(Event::Roll(vec![6, 10, 20]), effect_callback);
        assert!(effects.is_empty());

        let Ok(mut effects) = effects_rx.recv() else {
            panic!()
        };

        let NarrowEffect::Render(request) = effects.remove(0);
        let render_operation = request.operation;

        assert_eq!(RenderOperation, render_operation);
    }

    #[effect]
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

    struct BincodeFfiFormat;

    impl BincodeFfiFormat {
        fn bincode_options(
        ) -> WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>
        {
            DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes()
        }
    }

    impl FfiFormat for BincodeFfiFormat {
        type Serializer<'b> = bincode::Serializer<
            &'b mut Vec<u8>,
            WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
        >;
        type Deserializer<'b> = bincode::Deserializer<
            SliceReader<'b>,
            WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
        >;

        fn serializer(
            buffer: &mut Vec<u8>,
        ) -> bincode::Serializer<
            &'_ mut Vec<u8>,
            WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
        > {
            bincode::Serializer::new(buffer, Self::bincode_options())
        }

        fn deserializer(
            bytes: &[u8],
        ) -> bincode::Deserializer<
            SliceReader<'_>,
            WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
        > {
            bincode::Deserializer::from_slice(bytes, Self::bincode_options())
        }
    }

    #[test]
    fn roll_three_dice_with_type_narrowing_and_bridge() -> anyhow::Result<()> {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects| effects_tx.send(effects).unwrap();

        let inner_core: Core<Dice> = Core::new();
        let core = inner_core
            .handle_effects_using(RngMiddleware::new())
            .map_effect::<BridgeEffect>()
            .bridge::<BincodeFfiFormat>(effect_callback);

        let event: Vec<u8> = bincode::serialize(&Event::Roll(vec![6, 10, 20]))?;

        let effect_bytes = core.process_event(&event)?;
        let effects: Vec<bridge::Request<BridgeEffectFfi>> = bincode::deserialize(&effect_bytes)?;

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

        let effect_bytes = core.resolve(effect_id, &response_bytes)?;
        let mut effects: Vec<bridge::Request<BridgeEffectFfi>> =
            bincode::deserialize(&effect_bytes)?;

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

    struct JsonFfiFormat;

    impl FfiFormat for JsonFfiFormat {
        type Serializer<'b> = serde_json::Serializer<&'b mut Vec<u8>>;
        type Deserializer<'b> = serde_json::Deserializer<serde_json::de::SliceRead<'b>>;

        fn serializer(buffer: &mut Vec<u8>) -> serde_json::Serializer<&mut Vec<u8>> {
            serde_json::Serializer::new(buffer)
        }

        fn deserializer(bytes: &[u8]) -> serde_json::Deserializer<serde_json::de::SliceRead<'_>> {
            serde_json::Deserializer::from_slice(bytes)
        }
    }

    #[test]
    fn roll_three_dice_with_type_narrowing_and_json_bridge() -> anyhow::Result<()> {
        let (effects_tx, effects_rx) = crossbeam_channel::unbounded();
        let effect_callback = move |effects| effects_tx.send(effects).unwrap();

        let inner_core: Core<Dice> = Core::new();
        let core = inner_core
            .handle_effects_using(RngMiddleware::new())
            .map_effect::<BridgeEffect>()
            .bridge::<JsonFfiFormat>(effect_callback);

        let event = serde_json::to_vec(&Event::Roll(vec![6, 10, 20]))?;

        let effects_bytes = core.process_event(&event)?;
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

        let bridge::Request {
            effect: BridgeEffectFfi::Http(_),
            id: effect_id,
        } = effects.remove(0)
        else {
            panic!("Expected a HTTP request")
        };

        let response = HttpResult::Ok(HttpResponse::status(201).build());
        let response_bytes = serde_json::to_vec(&response)?;

        let effects_bytes = core.resolve(effect_id, &response_bytes)?;

        assert_eq!(
            str::from_utf8(&effects_bytes)?,
            r#"[{"id":0,"effect":{"Render":null}}]"#
        );

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
}
