#[cfg(feature = "chrono")]
mod shared {
    use chrono::{DateTime, Utc};
    use crux_core::render::Render;
    use crux_core::{macros::Effect, Command};
    use crux_time::{Time, TimeResponse};
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        Get,
        GetAsync,
        Set(TimeResponse),

        StartDebounce,
        DurationElapsed(usize, TimeResponse),
    }

    #[derive(Default)]
    struct Debounce {
        pending: usize,
    }

    impl Debounce {
        fn start(&mut self) -> usize {
            self.pending = self.pending.wrapping_add(1);
            self.pending
        }

        fn resolve(&mut self, pending: usize) -> bool {
            self.pending == pending
        }
    }

    #[derive(Default)]
    pub struct Model {
        pub time: String,
        debounce: Debounce,
        pub debounce_complete: bool,
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

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) -> Command<Event> {
            match event {
                Event::Get => caps.time.now(Event::Set),
                Event::GetAsync => {
                    let time = caps.time.clone();
                    Command::effect(
                        async move { Command::event(Event::Set(time.now_async().await)) },
                    )
                }
                Event::Set(time) => {
                    if let TimeResponse::Now(time) = time {
                        let time: DateTime<Utc> = time.try_into().unwrap();
                        model.time = time.to_rfc3339();
                        caps.render.render()
                    } else {
                        Command::none()
                    }
                }
                Event::StartDebounce => {
                    let pending = model.debounce.start();

                    caps.time.notify_after(
                        crux_time::Duration::from_millis(300).expect("valid duration"),
                        event_with_user_info(pending, Event::DurationElapsed),
                    )
                }
                Event::DurationElapsed(pending, TimeResponse::DurationElapsed) => {
                    if model.debounce.resolve(pending) {
                        model.debounce_complete = true;
                    }
                    Command::none()
                }
                Event::DurationElapsed(_, _) => {
                    panic!("Unexpected debounce event")
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
        pub time: Time,
        pub render: Render,
    }

    /// Helper to create an event with additional user info captured
    /// this is effectively partially applying the event constructor
    pub fn event_with_user_info<E, F, U, T>(user_info: U, make_event: F) -> impl FnOnce(T) -> E
    where
        F: FnOnce(U, T) -> E,
    {
        move |response| make_event(user_info, response)
    }
}

#[cfg(feature = "chrono")]
mod shell {
    use super::shared::{App, Effect, Event};
    use chrono::{DateTime, Utc};
    use crux_core::{Core, Request};
    use crux_time::{Instant, TimeRequest, TimeResponse};
    use std::collections::VecDeque;

    pub enum Outcome {
        Time(Request<TimeRequest>, Instant),
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
                    core.resolve(&mut request, TimeResponse::Now(result))
                }
                _ => vec![],
            };

            for effect in effs {
                if let Effect::Time(request) = effect {
                    let time: DateTime<Utc> =
                        "2022-12-01T01:47:12.746202562+00:00".parse().unwrap();
                    queue.push_back(CoreMessage::Response(Outcome::Time(
                        request,
                        time.try_into().unwrap(),
                    )));
                }
            }
        }
    }
}

#[cfg(feature = "chrono")]
mod tests {
    use crate::{
        shared::{App, Effect, Event, Model},
        shell::run,
    };
    use chrono::{DateTime, Utc};
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

        let now: DateTime<Utc> = "2022-12-01T01:47:12.746202562+00:00".parse().unwrap();
        let response = TimeResponse::Now(now.try_into().unwrap());
        let update = app.resolve(&mut request, response).unwrap();

        let event = update.events.into_iter().next().unwrap();
        app.update(event, &mut model);

        assert_eq!(app.view(&model).time, "2022-12-01T01:47:12.746202562+00:00");
    }

    #[test]
    pub fn test_debounce_timer() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update1 = app.update(Event::StartDebounce, &mut model);
        let update2 = app.update(Event::StartDebounce, &mut model);

        let Effect::Time(mut request1) = update1.into_effects().next().unwrap() else {
            panic!("Expected Time effect");
        };

        // resolve and run loop
        app.update(
            app.resolve(&mut request1, TimeResponse::DurationElapsed)
                .unwrap()
                .events
                .into_iter()
                .next()
                .unwrap(),
            &mut model,
        );

        // resolving the first debounce should not set the debounce_complete flag
        assert!(!model.debounce_complete);

        let Effect::Time(mut request2) = update2.into_effects().next().unwrap() else {
            panic!("Expected Time effect");
        };

        // resolve and run loop
        app.update(
            app.resolve(&mut request2, TimeResponse::DurationElapsed)
                .unwrap()
                .events
                .into_iter()
                .next()
                .unwrap(),
            &mut model,
        );

        // resolving the second debounce should set the debounce_complete flag
        assert!(model.debounce_complete);
    }
}
