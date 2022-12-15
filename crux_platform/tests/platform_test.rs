mod shared {
    use crux_core::{capability::CapabilityContext, render::Render, App, WithContext};
    use crux_platform::{Platform, PlatformResponse};
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct MyApp;

    #[derive(Serialize, Deserialize)]
    pub enum MyEvent {
        PlatformGet,
        PlatformSet(PlatformResponse),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct MyModel {
        pub platform: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct MyViewModel {
        pub platform: String,
    }

    impl App for MyApp {
        type Event = MyEvent;
        type Model = MyModel;
        type ViewModel = MyViewModel;
        type Capabilities = MyCapabilities;

        fn update(&self, event: MyEvent, model: &mut MyModel, caps: &MyCapabilities) {
            match event {
                MyEvent::PlatformGet => caps.platform.get(MyEvent::PlatformSet),
                MyEvent::PlatformSet(platform) => {
                    model.platform = platform.0;
                    caps.render.render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            MyViewModel {
                platform: model.platform.clone(),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum MyEffect {
        Platform,
        Render,
    }

    pub struct MyCapabilities {
        pub platform: Platform<MyEvent>,
        pub render: Render<MyEvent>,
    }

    impl WithContext<MyApp, MyEffect> for MyCapabilities {
        fn new_with_context(context: CapabilityContext<MyEffect, MyEvent>) -> MyCapabilities {
            MyCapabilities {
                platform: Platform::new(context.with_effect(|_| MyEffect::Platform)),
                render: Render::new(context.with_effect(|_| MyEffect::Render)),
            }
        }
    }
}

mod shell {
    use super::shared::{MyApp, MyEffect, MyEvent, MyViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_platform::PlatformResponse;
    use std::collections::VecDeque;

    pub enum Outcome {
        Platform(PlatformResponse),
    }

    enum CoreMessage {
        Message(MyEvent),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<MyEffect>, MyViewModel)> {
        let core: Core<MyEffect, MyApp> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(MyEvent::PlatformGet));

        let mut received = vec![];

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let reqs = match msg {
                Some(CoreMessage::Message(m)) => core.message(&bcs::to_bytes(&m)?),
                Some(CoreMessage::Response(uuid, output)) => core.response(
                    &uuid,
                    &match output {
                        Outcome::Platform(x) => bcs::to_bytes(&x)?,
                    },
                ),
                _ => vec![],
            };
            let reqs: Vec<Request<MyEffect>> = bcs::from_bytes(&reqs)?;

            for Request { uuid, effect } in reqs {
                match effect {
                    MyEffect::Render => received.push(effect),
                    MyEffect::Platform => {
                        received.push(effect);
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::Platform(PlatformResponse("test shell".to_string())),
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
    pub fn test_platform() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(received, vec![MyEffect::Platform, MyEffect::Render]);
        assert_eq!(view.platform, "test shell");
        Ok(())
    }
}
