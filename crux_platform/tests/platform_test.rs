mod shared {
    use crux_core::{render::Render};
    use crux_macros::Effect;
    use crux_platform::{Platform, PlatformResponse};
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        PlatformGet,
        PlatformSet(PlatformResponse),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct Model {
        pub platform: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct ViewModel {
        pub platform: String,
    }

    impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {
            match event {
                Event::PlatformGet => caps.platform.get(Event::PlatformSet),
                Event::PlatformSet(platform) => {
                    model.platform = platform.0;
                    caps.render.render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                platform: model.platform.clone(),
            }
        }
    }

    #[derive(Effect)]
    pub struct Capabilities {
        pub platform: Platform<Event>,
        pub render: Render<Event>,
    }
}

mod shell {
    use super::shared::{App, Effect, Event, ViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_platform::PlatformResponse;
    use std::collections::VecDeque;

    pub enum Outcome {
        Platform(PlatformResponse),
    }

    enum CoreMessage {
        Message(Event),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<Effect>, ViewModel)> {
        let core: Core<Effect, App> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(Event::PlatformGet));

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

            for Request { uuid, effect } in reqs {
                match effect {
                    Effect::Render(_) => received.push(effect),
                    Effect::Platform(_) => {
                        received.push(effect);
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
    use crate::{shared::Effect, shell::run};
    use anyhow::Result;
    use crux_core::render::RenderOperation;
    use crux_platform::PlatformRequest;

    #[test]
    pub fn test_platform() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(
            received,
            vec![
                Effect::Platform(PlatformRequest),
                Effect::Render(RenderOperation)
            ]
        );
        assert_eq!(view.platform, "test shell");
        Ok(())
    }
}
