mod shared {
    use crux_core::{render::Render, App, Capabilities, Command};
    use crux_platform::{Platform, PlatformResponse};
    use serde::{Deserialize, Serialize};
    use std::marker::PhantomData;

    #[derive(Default)]
    pub struct MyApp<Ef, Caps> {
        _marker: PhantomData<fn() -> (Ef, Caps)>,
    }

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

    impl<Ef, Caps> App<Ef, Caps> for MyApp<Ef, Caps>
    where
        Ef: Serialize + Clone + Default,
        Caps: Default + Capabilities<Platform<Ef>> + Capabilities<Render<Ef>>,
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
            let platform = <Caps as crux_core::Capabilities<Platform<_>>>::get(caps);
            let render = <Caps as crux_core::Capabilities<Render<_>>>::get(caps);

            match event {
                MyEvent::PlatformGet => {
                    vec![platform.get(MyEvent::PlatformSet)]
                }
                MyEvent::PlatformSet(platform) => {
                    model.platform = platform.0;
                    vec![render.render()]
                }
            }
        }

        fn view(
            &self,
            model: &<Self as App<Ef, Caps>>::Model,
        ) -> <Self as App<Ef, Caps>>::ViewModel {
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

    impl Default for MyEffect {
        fn default() -> Self {
            MyEffect::Render
        }
    }

    pub(crate) struct MyCapabilities {
        pub platform: Platform<MyEffect>,
        pub render: Render<MyEffect>,
    }

    impl crux_core::Capabilities<Platform<MyEffect>> for MyCapabilities {
        fn get(&self) -> &Platform<MyEffect> {
            &self.platform
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
                platform: Platform::new(MyEffect::Platform),
                render: Render::new(MyEffect::Render),
            }
        }
    }
}

mod shell {
    use super::shared::{MyApp, MyCapabilities, MyEffect, MyEvent, MyViewModel};
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
        let core: Core<MyEffect, MyCapabilities, MyApp<MyEffect, MyCapabilities>> = Core::new();
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

            for req in reqs {
                let Request { uuid, effect } = req;
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
