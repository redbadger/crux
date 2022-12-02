mod shared {
    use crux_core::{render::Render, App, Capabilities, Command};
    use crux_kv::{KeyValue, KeyValueRequest, KeyValueResponse};
    use serde::{Deserialize, Serialize};
    use std::marker::PhantomData;

    #[derive(Default)]
    pub struct MyApp<Ef, Caps> {
        _marker: PhantomData<fn() -> (Ef, Caps)>,
    }

    #[derive(Serialize, Deserialize)]
    pub enum MyEvent {
        Write,
        Read,
        Set(KeyValueResponse),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct MyModel {
        pub value: i32,
        pub successful: bool,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct MyViewModel {
        pub result: String,
    }

    impl<Ef, Caps> App<Ef, Caps> for MyApp<Ef, Caps>
    where
        Ef: Serialize + Clone + Default,
        Caps: Default + Capabilities<KeyValue<Ef>> + Capabilities<Render<Ef>>,
    {
        type Event = MyEvent;
        type Model = MyModel;
        type ViewModel = MyViewModel;

        fn update(
            &self,
            event: MyEvent,
            model: &mut MyModel,
            caps: &Caps,
        ) -> Vec<Command<Ef, MyEvent>> {
            let key_value: &KeyValue<_> = caps.get();
            let render: &Render<_> = caps.get();

            match event {
                MyEvent::Write => {
                    vec![key_value.write("test", 42i32.to_ne_bytes().to_vec(), MyEvent::Set)]
                }
                MyEvent::Set(KeyValueResponse::Write(success)) => {
                    model.successful = success;
                    vec![render.render()]
                }
                MyEvent::Read => vec![key_value.read("test", MyEvent::Set)],
                MyEvent::Set(KeyValueResponse::Read(value)) => {
                    if let Some(value) = value {
                        // TODO: should KeyValueResponse::Read be generic over the value type?
                        let (int_bytes, _rest) = value.split_at(std::mem::size_of::<i32>());
                        model.value = i32::from_ne_bytes(int_bytes.try_into().unwrap());
                    }
                    vec![render.render()]
                }
            }
        }

        fn view(
            &self,
            model: &<Self as App<Ef, Caps>>::Model,
        ) -> <Self as App<Ef, Caps>>::ViewModel {
            MyViewModel {
                result: format!("Success: {}, Value: {}", model.successful, model.value),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum MyEffect {
        KeyValue(KeyValueRequest),
        Render,
    }

    impl Default for MyEffect {
        fn default() -> Self {
            MyEffect::Render
        }
    }

    pub(crate) struct MyCapabilities {
        pub key_value: KeyValue<MyEffect>,
        pub render: Render<MyEffect>,
    }

    impl crux_core::Capabilities<KeyValue<MyEffect>> for MyCapabilities {
        fn get(&self) -> &KeyValue<MyEffect> {
            &self.key_value
        }
    }

    impl crux_core::Capabilities<Render<MyEffect>> for MyCapabilities {
        fn get(&self) -> &Render<MyEffect> {
            &self.render
        }
    }

    impl Default for MyCapabilities {
        fn default() -> Self {
            Self {
                key_value: KeyValue::new(MyEffect::KeyValue),
                render: Render::new(MyEffect::Render),
            }
        }
    }
}

mod shell {
    use super::shared::{MyApp, MyCapabilities, MyEffect, MyEvent, MyViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_kv::{KeyValueRequest, KeyValueResponse};
    use std::collections::{HashMap, VecDeque};

    pub enum Outcome {
        KeyValue(KeyValueResponse),
    }

    enum CoreMessage {
        Message(MyEvent),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<MyEffect>, MyViewModel)> {
        let core: Core<MyEffect, MyCapabilities, MyApp<MyEffect, MyCapabilities>> = Core::new();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(MyEvent::Write));

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
            let reqs: Vec<Request<MyEffect>> = bcs::from_bytes(&reqs)?;

            for req in reqs {
                let Request { uuid, effect } = req;
                match effect {
                    MyEffect::Render => received.push(effect.clone()),
                    MyEffect::KeyValue(KeyValueRequest::Write(ref k, ref v)) => {
                        received.push(effect.clone());

                        // do work
                        kv_store.insert(k.clone(), v.clone());
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::KeyValue(KeyValueResponse::Write(true)),
                        ));

                        // now trigger a read
                        queue.push_back(CoreMessage::Message(MyEvent::Read));
                    }
                    MyEffect::KeyValue(KeyValueRequest::Read(ref k)) => {
                        received.push(effect.clone());

                        // do work
                        let v = kv_store.get(k).unwrap();
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::KeyValue(KeyValueResponse::Read(Some(v.to_vec()))),
                        ));
                    }
                }
            }
        }

        let view = bcs::from_bytes::<MyViewModel>(&core.view())?;
        Ok((received, view))
    }
}

mod tests {
    use crate::{shared::MyEffect, shell::run};
    use anyhow::Result;
    use crux_kv::KeyValueRequest;

    #[test]
    pub fn test_http() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(
            received,
            vec![
                MyEffect::KeyValue(KeyValueRequest::Write(
                    "test".to_string(),
                    42i32.to_ne_bytes().to_vec()
                )),
                MyEffect::Render,
                MyEffect::KeyValue(KeyValueRequest::Read("test".to_string())),
                MyEffect::Render
            ]
        );
        assert_eq!(view.result, "Success: true, Value: 42");
        Ok(())
    }
}
