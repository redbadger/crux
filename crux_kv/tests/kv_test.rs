mod shared {
    use crux_core::macros::Effect;
    use crux_core::render::Render;
    use crux_kv::{KeyValue, KeyValueOutput};
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Read,
        Write,
        Delete,
        ReadThenWrite,
        Set(KeyValueOutput),
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    pub struct Model {
        pub value: i32,
        pub successful: bool,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct ViewModel {
        pub result: String,
    }

    impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;

        type Capabilities = Capabilities;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {
            println!("Update: {event:?}. Model: {model:?}");

            match event {
                Event::Read => caps.key_value.get("test", Event::Set),
                Event::Set(KeyValueOutput::Get { value }) => {
                    if let Some(value) = value.unwrap() {
                        // TODO: should KeyValueOutput::Get be generic over the value type?
                        let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                        model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());
                    }
                    caps.render.render()
                }
                Event::Write => {
                    caps.key_value
                        .set("test", 42i32.to_ne_bytes().to_vec(), Event::Set);
                }
                Event::Set(KeyValueOutput::Set { result })
                | Event::Set(KeyValueOutput::Delete { result }) => {
                    model.successful = result.is_ok();
                    caps.render.render()
                }
                Event::ReadThenWrite => caps.compose.spawn(|ctx| {
                    let kv = caps.key_value.clone();

                    async move {
                        let KeyValueOutput::Get { value } = kv.get_async("test_num").await else {
                            panic!("Expected read and got write");
                        };

                        let Some(value) = value.unwrap() else {
                            panic!("Read failed;");
                        };

                        let num = i32::from_ne_bytes(value.try_into().unwrap());
                        let result = kv
                            .set_async("test_num", (num + 1).to_ne_bytes().to_vec())
                            .await;

                        ctx.update_app(Event::Set(result))
                    }
                }),
                Event::Delete => {
                    caps.key_value.delete("test", Event::Set);
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                result: format!("Success: {}, Value: {}", model.successful, model.value),
            }
        }
    }

    #[derive(Effect)]
    pub struct Capabilities {
        pub key_value: KeyValue<Event>,
        pub render: Render<Event>,
        #[effect(skip)]
        pub compose: crux_core::compose::Compose<Event>,
    }
}

mod shell {
    use super::shared::{App, Effect, Event};
    use crux_core::{Core, Request};
    use crux_kv::{KeyValueOperation, KeyValueOutput};
    use std::collections::{HashMap, VecDeque};

    #[derive(Debug)]
    pub enum Outcome {
        KeyValue(Request<KeyValueOperation>, KeyValueOutput),
    }

    #[derive(Debug)]
    enum CoreMessage {
        Event(Event),
        Response(Outcome),
    }

    pub fn run(core: &Core<Effect, App>) {
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Event(Event::Write));

        let mut kv_store = HashMap::new();

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let effects = match msg {
                Some(CoreMessage::Event(m)) => core.process_event(m),
                Some(CoreMessage::Response(Outcome::KeyValue(mut request, output))) => {
                    core.resolve(&mut request, output)
                }
                _ => vec![],
            };

            for effect in effects {
                match effect {
                    Effect::Render(_) => (),
                    Effect::KeyValue(request) => {
                        match request.operation {
                            KeyValueOperation::Set { ref key, ref value } => {
                                // received.push(effect);

                                // do work
                                kv_store.insert(key.clone(), value.clone());

                                queue.push_back(CoreMessage::Response(Outcome::KeyValue(
                                    request,
                                    KeyValueOutput::Set { result: Ok(()) },
                                )));

                                // now trigger a read
                                queue.push_back(CoreMessage::Event(Event::Read));
                            }
                            KeyValueOperation::Get { ref key } => {
                                // received.push(effect);

                                // do work
                                let value = Ok(Some(kv_store.get(key).unwrap().to_vec()));
                                queue.push_back(CoreMessage::Response(Outcome::KeyValue(
                                    request,
                                    KeyValueOutput::Get { value },
                                )));

                                // now trigger a delete
                                queue.push_back(CoreMessage::Event(Event::Delete));
                            }
                            KeyValueOperation::Delete { ref key } => {
                                // received.push(effect);

                                // do work
                                kv_store.remove(key);

                                queue.push_back(CoreMessage::Response(Outcome::KeyValue(
                                    request,
                                    KeyValueOutput::Delete { result: Ok(()) },
                                )));
                            }
                        }
                    }
                }
            }
        }
    }
}

mod tests {
    use crate::{
        shared::App,
        shared::{Effect, Event, Model},
        shell::run,
    };
    use anyhow::Result;
    use crux_core::{testing::AppTester, Core};
    use crux_kv::{KeyValueOperation, KeyValueOutput};

    #[test]
    pub fn test_kv() -> Result<()> {
        let core: Core<Effect, App> = Core::default();

        assert_eq!(core.view().result, "Success: false, Value: 0");

        run(&core);

        assert_eq!(core.view().result, "Success: true, Value: 42");

        Ok(())
    }

    #[test]
    pub fn test_kv_async() -> Result<()> {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::ReadThenWrite, &mut model);

        let effect = update.into_effects().next().unwrap();
        let Effect::KeyValue(mut request) = effect else {
            panic!("Expected KeyValue effect");
        };

        let KeyValueOperation::Get { key } = request.operation.clone() else {
            panic!("Expected read operation");
        };

        assert_eq!(key, "test_num");

        let update = app
            .resolve(
                &mut request,
                KeyValueOutput::Get {
                    value: Ok(Some(17u32.to_ne_bytes().to_vec())),
                },
            )
            .unwrap();

        let effect = update.into_effects().next().unwrap();
        let Effect::KeyValue(mut request) = effect else {
            panic!("Expected KeyValue effect");
        };

        let KeyValueOperation::Set { key, value } = request.operation.clone() else {
            panic!("Expected read operation");
        };

        assert_eq!(key, "test_num".to_string());
        assert_eq!(value, 18u32.to_ne_bytes().to_vec());

        let update = app
            .resolve(&mut request, KeyValueOutput::Set { result: Ok(()) })
            .unwrap();

        let event = update.events.into_iter().next().unwrap();
        app.update(event, &mut model);

        assert!(model.successful);

        Ok(())
    }
}
