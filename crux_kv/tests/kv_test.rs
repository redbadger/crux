mod shared {
    use crux_core::{capability::CapabilityContext, render::Render};
    use crux_kv::{KeyValue, KeyValueOperation, KeyValueOutput};
    use crux_macros::Effect;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        Write,
        Read,
        Set(KeyValueOutput),
    }

    #[derive(Default, Serialize, Deserialize)]
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
        #[effect(operation = "KeyValueOperation")]
        pub key_value: KeyValue<Event>,
        pub render: Render<Event>,
    }
}

mod shell {
    use super::shared::{App, Effect, Event, ViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_kv::{KeyValueOperation, KeyValueOutput};
    use std::collections::{HashMap, VecDeque};

    pub enum Outcome {
        KeyValue(KeyValueOutput),
    }

    enum CoreMessage {
        Message(Event),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<Effect>, ViewModel)> {
        let core: Core<Effect, App> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(Event::Write));

        let mut received = vec![];
        let mut kv_store = HashMap::new();

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let reqs = match msg {
                Some(CoreMessage::Message(m)) => core.message(&bcs::to_bytes(&m)?),
                Some(CoreMessage::Response(uuid, output)) => core.response(
                    &uuid,
                    &match output {
                        Outcome::KeyValue(x) => bcs::to_bytes(&x)?,
                    },
                ),
                _ => vec![],
            };
            let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs)?;

            for Request { uuid, effect } in reqs {
                match effect {
                    Effect::Render => received.push(effect.clone()),
                    Effect::KeyValue(KeyValueOperation::Write(ref k, ref v)) => {
                        received.push(effect.clone());

                        // do work
                        kv_store.insert(k.clone(), v.clone());
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::KeyValue(KeyValueOutput::Write(true)),
                        ));

                        // now trigger a read
                        queue.push_back(CoreMessage::Message(Event::Read));
                    }
                    Effect::KeyValue(KeyValueOperation::Read(ref k)) => {
                        received.push(effect.clone());

                        // do work
                        let v = kv_store.get(k).unwrap();
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::KeyValue(KeyValueOutput::Read(Some(v.to_vec()))),
                        ));
                    }
                }
            }
        }

        let view = bcs::from_bytes::<ViewModel>(&core.view())?;
        Ok((received, view))
    }
}

mod tests {
    use crate::{shared::Effect, shell::run};
    use anyhow::Result;
    use crux_kv::KeyValueOperation;

    #[test]
    pub fn test_http() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(
            received,
            vec![
                Effect::KeyValue(KeyValueOperation::Write(
                    "test".to_string(),
                    42i32.to_ne_bytes().to_vec()
                )),
                Effect::Render,
                Effect::KeyValue(KeyValueOperation::Read("test".to_string())),
                Effect::Render
            ]
        );
        assert_eq!(view.result, "Success: true, Value: 42");
        Ok(())
    }
}
