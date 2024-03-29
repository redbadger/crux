mod shared {
    use crux_core::macros::Effect;
    use crux_core::render::Render;
    use crux_time::Time;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        Get,
        GetAsync,
        Set(chrono::DateTime<chrono::Utc>),
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
                Event::Get => caps.time.now(Event::Set),
                Event::GetAsync => caps.compose.spawn(|ctx| {
                    let time = caps.time.clone();

                    async move {
                        ctx.update_app(Event::Set(time.now_async().await));
                    }
                }),
                Event::Set(time) => {
                    model.time = time.to_rfc3339();
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
        #[effect(skip)]
        pub compose: crux_core::compose::Compose<Event>,
    }
}

mod shell {
    use super::shared::{App, Effect, Event};
    use crux_core::{Core, Request};
    use crux_time::{TimeRequest, TimeResponse};
    use std::collections::VecDeque;

    pub enum Outcome {
        Time(Request<TimeRequest>, String),
    }

    enum CoreMessage {
        Event(Event),
        Response(Outcome),
    }

    pub fn run(core: &Core<Effect, App>) {
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Event(Event::Get));

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let effs = match msg {
                Some(CoreMessage::Event(m)) => core.process_event(m),
                Some(CoreMessage::Response(Outcome::Time(mut request, result))) => {
                    core.resolve(&mut request, TimeResponse(result))
                }
                _ => vec![],
            };

            for effect in effs {
                if let Effect::Time(request) = effect {
                    queue.push_back(CoreMessage::Response(Outcome::Time(
                        request,
                        "2022-12-01T01:47:12.746202562+00:00".to_string(),
                    )));
                }
            }
        }
    }
}

mod tests {
    use crate::{
        shared::{App, Effect, Event, Model},
        shell::run,
    };
    use crux_core::{testing::AppTester, Core};
    use crux_time::TimeResponse;

    #[test]
    pub fn test_time() {
        let core: Core<Effect, App> = Core::default();

        run(&core);

        assert_eq!(core.view().time, "2022-12-01T01:47:12.746202562+00:00");
    }

    #[test]
    pub fn test_time_async() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::GetAsync, &mut model);

        let effect = update.into_effects().next().unwrap();
        let Effect::Time(mut request) = effect else {
            panic!("Expected Time effect");
        };

        let now = "2022-12-01T01:47:12.746202562+00:00".to_string();
        let response = TimeResponse(now);
        let update = app.resolve(&mut request, response).unwrap();

        let event = update.events.into_iter().next().unwrap();
        app.update(event, &mut model);

        assert_eq!(app.view(&model).time, "2022-12-01T01:47:12.746202562+00:00");
    }
}
