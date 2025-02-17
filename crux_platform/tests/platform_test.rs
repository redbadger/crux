mod shared {
    use crux_core::render::Render;
    use crux_core::{macros::Effect, Command};
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
        type Effect = Effect;

        fn update(
            &self,
            event: Event,
            model: &mut Model,
            caps: &Capabilities,
        ) -> Command<Effect, Event> {
            match event {
                Event::PlatformGet => caps.platform.get(Event::PlatformSet),
                Event::PlatformSet(platform) => {
                    model.platform = platform.0;
                    caps.render.render()
                }
            }

            Command::done()
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
    use super::shared::{App, Effect, Event};
    use crux_core::{Core, Request};
    use crux_platform::{PlatformRequest, PlatformResponse};
    use std::collections::VecDeque;

    pub enum Outcome {
        Platform(Request<PlatformRequest>, PlatformResponse),
    }

    enum CoreMessage {
        Event(Event),
        Response(Outcome),
    }

    pub fn run(core: &Core<App>) {
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Event(Event::PlatformGet));

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let effs = match msg {
                Some(CoreMessage::Event(m)) => core.process_event(m),
                Some(CoreMessage::Response(Outcome::Platform(mut request, outcome))) => core
                    .resolve(&mut request, outcome)
                    .expect("effect should resolve"),

                _ => vec![],
            };

            for effect in effs {
                if let Effect::Platform(request) = effect {
                    queue.push_back(CoreMessage::Response(Outcome::Platform(
                        request,
                        PlatformResponse("test shell".to_string()),
                    )));
                }
            }
        }
    }
}

mod tests {
    use crate::{shared::App, shell::run};
    use crux_core::Core;

    #[test]
    pub fn test_platform() {
        let core: Core<App> = Core::default();

        run(&core);

        assert_eq!(core.view().platform, "test shell");
    }
}
