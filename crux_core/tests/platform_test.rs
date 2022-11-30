mod shared {
    use crux_core::{
        platform::{Platform, PlatformResponse},
        render::Render,
        App, Capabilities, Command,
    };
    use serde::{Deserialize, Serialize};
    use std::marker::PhantomData;

    #[derive(Default)]
    pub struct PlatformApp<Ef, Caps> {
        _marker: PhantomData<fn() -> (Ef, Caps)>,
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct PlatformModel {
        pub platform: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct ViewModel {
        pub platform: String,
    }

    #[derive(Serialize, Deserialize)]
    pub enum PlatformEvent {
        Get,
        Set(PlatformResponse),
    }

    impl<Ef, Caps> App<Ef, Caps> for PlatformApp<Ef, Caps>
    where
        Ef: Serialize + Clone + Default,
        Caps: Default + Capabilities<Platform<Ef>> + Capabilities<Render<Ef>>,
    {
        type Event = PlatformEvent;
        type Model = PlatformModel;
        type ViewModel = PlatformModel;

        fn update(
            &self,
            msg: PlatformEvent,
            model: &mut PlatformModel,
            caps: &Caps,
        ) -> Vec<Command<Ef, PlatformEvent>> {
            let platform = <Caps as crux_core::Capabilities<Platform<_>>>::get(caps);
            let render = <Caps as crux_core::Capabilities<Render<_>>>::get(caps);

            match msg {
                PlatformEvent::Get => {
                    vec![platform.get(PlatformEvent::Set)]
                }
                PlatformEvent::Set(platform) => {
                    model.platform = platform.0;
                    vec![render.render()]
                }
            }
        }

        fn view(
            &self,
            model: &<Self as App<Ef, Caps>>::Model,
        ) -> <Self as App<Ef, Caps>>::ViewModel {
            PlatformModel {
                platform: model.platform.clone(),
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        GetPlatform,
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub enum Effect {
        Platform,
        Render,
    }

    impl Default for Effect {
        fn default() -> Self {
            Effect::Render
        }
    }

    pub(crate) struct MyCapabilities {
        pub platform: Platform<Effect>,
        pub render: Render<Effect>,
    }

    impl crux_core::Capabilities<Platform<Effect>> for MyCapabilities {
        fn get(&self) -> &Platform<Effect> {
            &self.platform
        }
    }

    impl crux_core::Capabilities<Render<Effect>> for MyCapabilities {
        fn get(&self) -> &Render<Effect> {
            &self.render
        }
    }

    impl Default for MyCapabilities {
        fn default() -> Self {
            Self {
                platform: Platform::new(Effect::Platform),
                render: Render::new(Effect::Render),
            }
        }
    }
}

mod shell {
    use super::shared::{Effect, Event, MyCapabilities, PlatformApp, ViewModel};
    use anyhow::Result;
    use crux_core::{platform::PlatformResponse, Core, Request};
    use std::collections::VecDeque;

    pub enum Outcome {
        Platform(PlatformResponse),
    }

    enum CoreMessage {
        Message(Event),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<&'static str>, ViewModel)> {
        let core: Core<Effect, MyCapabilities, PlatformApp<Effect, MyCapabilities>> = Core::new();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(Event::GetPlatform));

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
            let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs)?;

            for req in reqs {
                let Request { uuid, effect } = req;
                match effect {
                    Effect::Render => received.push("render"),
                    Effect::Platform => {
                        received.push("platform");
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::Platform(PlatformResponse("test shell".to_string())),
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
    use crate::shell::run;
    use anyhow::Result;

    #[test]
    pub fn test_platform() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(received, ["platform", "render"]);
        assert_eq!(view.platform, "test shell");
        Ok(())
    }
}
