mod shared {
    use crux_core::render::Render;
    use crux_macros::Effect;
    use crux_time::{Time, TimeResponse};
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        TimeGet,
        TimeSet(TimeResponse),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct Model {
        pub time: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct ViewModel {
        pub time: String,
    }

    impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {
            match event {
                Event::TimeGet => caps.time.get(Event::TimeSet),
                Event::TimeSet(time) => {
                    model.time = time.0;
                    caps.render.render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                time: model.time.clone(),
            }
        }
    }

    #[derive(Effect)]
    pub struct Capabilities {
        pub time: Time<Event>,
        pub render: Render<Event>,
    }
}

mod shell {
    use super::shared::{App, Effect, Event};
    use crux_core::{Core, Request};
    use crux_time::{TimeRequest, TimeResponse};
    use std::collections::VecDeque;

    pub enum Outcome {
        Time(Request<TimeRequest>, TimeResponse),
    }

    enum CoreMessage {
        Event(Event),
        Response(Outcome),
    }

    pub fn run(core: &Core<Effect, App>) {
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Event(Event::TimeGet));

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let effs = match msg {
                Some(CoreMessage::Event(m)) => core.process_event(m),
                Some(CoreMessage::Response(Outcome::Time(mut request, result))) => {
                    core.resolve(&mut request, result)
                }
                _ => vec![],
            };

            for effect in effs {
                if let Effect::Time(request) = effect {
                    queue.push_back(CoreMessage::Response(Outcome::Time(
                        request,
                        TimeResponse("2022-12-01T01:47:12.746202562+00:00".to_string()),
                    )));
                }
            }
        }
    }
}

mod tests {
    use crate::{
        shared::{App, Effect},
        shell::run,
    };
    use crux_core::Core;

    #[test]
    pub fn test_time() {
        let core: Core<Effect, App> = Core::default();

        run(&core);

        assert_eq!(core.view().time, "2022-12-01T01:47:12.746202562+00:00");
    }
}
