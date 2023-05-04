mod shared {
    use crux_core::render::Render;
    use crux_kv::{KeyValue, KeyValueOutput};
    use crux_macros::Effect;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Write,
        Read,
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
                Event::Write => {
                    caps.key_value
                        .write("test", 42i32.to_ne_bytes().to_vec(), Event::Set);
                }
                Event::Set(KeyValueOutput::Write(success)) => {
                    model.successful = success;
                    caps.render.render()
                }
                Event::Read => caps.key_value.read("test", Event::Set),
                Event::Set(KeyValueOutput::Read(value)) => {
                    if let Some(value) = value {
                        // TODO: should KeyValueOutput::Read be generic over the value type?
                        let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                        model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());
                    }
                    caps.render.render()
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
                            KeyValueOperation::Write(ref k, ref v) => {
                                // received.push(effect);

                                // do work
                                kv_store.insert(k.clone(), v.clone());

                                queue.push_back(CoreMessage::Response(Outcome::KeyValue(
                                    request,
                                    KeyValueOutput::Write(true),
                                )));

                                // now trigger a read
                                queue.push_back(CoreMessage::Event(Event::Read));
                            }
                            KeyValueOperation::Read(ref k) => {
                                // received.push(effect);

                                // do work
                                let v = kv_store.get(k).unwrap();
                                queue.push_back(CoreMessage::Response(Outcome::KeyValue(
                                    request,
                                    KeyValueOutput::Read(Some(v.to_vec())),
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
    use crate::{shared::App, shared::Effect, shell::run};
    use anyhow::Result;
    use crux_core::Core;

    #[test]
    pub fn test_kv() -> Result<()> {
        let core: Core<Effect, App> = Core::default();

        assert_eq!(core.view().result, "Success: false, Value: 0");

        run(&core);

        assert_eq!(core.view().result, "Success: true, Value: 42");

        Ok(())
    }
}
