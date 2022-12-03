mod shared {
    use crux_core::{render::Render, App, CapabilityFactory, Command};
    use crux_time::{Time, TimeResponse};
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct MyApp;

    #[derive(Serialize, Deserialize)]
    pub enum MyEvent {
        TimeGet,
        TimeSet(TimeResponse),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct MyModel {
        pub time: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct MyViewModel {
        pub time: String,
    }

    impl App for MyApp {
        type Event = MyEvent;
        type Model = MyModel;
        type ViewModel = MyViewModel;
        type Capabilities = MyCapabilities;

        fn update(&self, event: MyEvent, model: &mut MyModel, caps: &MyCapabilities) {
            match event {
                MyEvent::TimeGet => caps.time.get(MyEvent::TimeSet),
                MyEvent::TimeSet(time) => {
                    model.time = time.0;
                    caps.render.render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            MyViewModel {
                time: model.time.clone(),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum MyEffect {
        Time,
        Render,
    }

    pub struct MyCapabilities {
        pub time: Time<MyEvent>,
        // TODO: Don't forget to fix Render
        pub render: Render<MyEvent, MyEffect>,
    }

    impl CapabilityFactory<MyApp, MyEffect> for MyCapabilities {
        fn build(
            channel: crux_core::channels::Sender<Command<MyEffect, MyEvent>>,
        ) -> MyCapabilities {
            MyCapabilities {
                time: Time::new(channel.map_effect(|_| MyEffect::Time)),
                render: Render::new(channel, || MyEffect::Render),
            }
        }
    }
}

mod shell {
    use super::shared::{MyApp, MyCapabilities, MyEffect, MyEvent, MyViewModel};
    use anyhow::Result;
    use crux_core::{CapabilityFactory, Core, Request};
    use crux_time::TimeResponse;
    use std::collections::VecDeque;

    pub enum Outcome {
        Time(TimeResponse),
    }

    enum CoreMessage {
        Message(MyEvent),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<MyEffect>, MyViewModel)> {
        let core: Core<MyEffect, MyApp> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(MyEvent::TimeGet));

        let mut received = vec![];

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let reqs = match msg {
                Some(CoreMessage::Message(m)) => core.message(&bcs::to_bytes(&m)?),
                Some(CoreMessage::Response(uuid, output)) => core.response(
                    &uuid,
                    &match output {
                        Outcome::Time(x) => bcs::to_bytes(&x)?,
                    },
                ),
                _ => vec![],
            };
            let reqs: Vec<Request<MyEffect>> = bcs::from_bytes(&reqs)?;

            for req in reqs {
                let Request { uuid, effect } = req;
                match effect {
                    MyEffect::Render => received.push(effect),
                    MyEffect::Time => {
                        received.push(effect);
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::Time(TimeResponse(
                                "2022-12-01T01:47:12.746202562+00:00".to_string(),
                            )),
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

    #[test]
    pub fn test_time() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(received, vec![MyEffect::Time, MyEffect::Render]);
        assert_eq!(view.time, "2022-12-01T01:47:12.746202562+00:00");
        Ok(())
    }
}
