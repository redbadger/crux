mod shared {
    use crux_core::{render::Render};
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
    use super::shared::{App, Effect, Event, ViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_time::TimeResponse;
    use std::collections::VecDeque;

    pub enum Outcome {
        Time(TimeResponse),
    }

    enum CoreMessage {
        Message(Event),
        Response(Vec<u8>, Outcome),
    }

    pub fn run() -> Result<(Vec<Effect>, ViewModel)> {
        let core: Core<Effect, App> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(Event::TimeGet));

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
            let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs)?;

            for Request { uuid, effect } in reqs {
                match effect {
                    Effect::Render(_) => received.push(effect),
                    Effect::Time(_) => {
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

        let view = bcs::from_bytes::<ViewModel>(&core.view())?;
        Ok((received, view))
    }
}

mod tests {
    use crate::{shared::Effect, shell::run};
    use anyhow::Result;
    use crux_core::render::RenderOperation;
    use crux_time::TimeRequest;

    #[test]
    pub fn test_time() -> Result<()> {
        let (received, view) = run()?;
        assert_eq!(
            received,
            vec![Effect::Time(TimeRequest), Effect::Render(RenderOperation)]
        );
        assert_eq!(view.time, "2022-12-01T01:47:12.746202562+00:00");
        Ok(())
    }
}
