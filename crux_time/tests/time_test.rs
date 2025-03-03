#[cfg(feature = "chrono")]
mod shared {
    use chrono::{DateTime, Utc};
    use crux_core::render::Render;
    use crux_core::{macros::Effect, Command};
    use crux_time::{
        protocol::{TimeResponse, TimerId},
        Time,
    };
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
        Cancel(TimerId),
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
        pub debounce_time_id: Option<TimerId>,
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
        type Effect = Effect;

        fn update(
            &self,
            event: Event,
            model: &mut Model,
            caps: &Capabilities,
        ) -> Command<Effect, Event> {
            match event {
                Event::Get => caps.time.now(Event::Set),
                Event::GetAsync => caps.compose.spawn(|ctx| {
                    let time = caps.time.clone();

                    async move {
                        ctx.update_app(Event::Set(time.now_async().await));
                    }
                }),
                Event::Set(time) => {
                    if let TimeResponse::Now { instant } = time {
                        let time: DateTime<Utc> = instant.try_into().unwrap();
                        model.time = time.to_rfc3339();
                        caps.render.render()
                    }
                }
                Event::StartDebounce => {
                    let pending = model.debounce.start();

                    let tid = caps.time.notify_after(
                        std::time::Duration::from_millis(300),
                        event_with_user_info(pending, Event::DurationElapsed),
                    );

                    model.debounce_time_id = Some(tid);
                }
                Event::DurationElapsed(pending, TimeResponse::DurationElapsed { id: _ }) => {
                    if model.debounce.resolve(pending) {
                        model.debounce_complete = true;
                    }
                }
                Event::DurationElapsed(_, TimeResponse::Cleared { id }) => {
                    if let Some(tid) = model.debounce_time_id {
                        if tid == id {
                            model.debounce_time_id = None;
                        }
                    }
                }
                Event::DurationElapsed(_, _) => {
                    panic!("Unexpected debounce event")
                }
                Event::Cancel(timer_id) => {
                    caps.time.clear(timer_id);
                }
            }

            Command::done()
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

    pub fn run(core: &Core<App>) {
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Event(Event::Get));

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let effs = match msg {
                Some(CoreMessage::Event(m)) => core.process_event(m),
                Some(CoreMessage::Response(Outcome::Time(mut request, instant))) => core
                    .resolve(&mut request, TimeResponse::Now { instant })
                    .expect("effect should resolve"),
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
        shared::{App, Event, Model},
        shell::run,
    };
    use chrono::{DateTime, Utc};
    use crux_core::{testing::AppTester, Core};
    use crux_time::protocol::TimeResponse;

    #[test]
    pub fn test_time() {
        let core: Core<App> = Core::default();

        run(&core);

        assert_eq!(core.view().time, "2022-12-01T01:47:12.746202562+00:00");
    }

    #[test]
    pub fn test_time_async() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request = &mut app
            .update(Event::GetAsync, &mut model)
            .expect_one_effect()
            .expect_time();

        let now: DateTime<Utc> = "2022-12-01T01:47:12.746202562+00:00".parse().unwrap();
        let response = TimeResponse::Now {
            instant: now.try_into().unwrap(),
        };
        let _update = app.resolve_to_event_then_update(request, response, &mut model);

        assert_eq!(app.view(&model).time, "2022-12-01T01:47:12.746202562+00:00");
    }

    #[test]
    pub fn test_debounce_timer() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request1 = &mut app
            .update(Event::StartDebounce, &mut model)
            .expect_one_effect()
            .expect_time();
        let request2 = &mut app
            .update(Event::StartDebounce, &mut model)
            .expect_one_effect()
            .expect_time();

        // resolve and update
        app.resolve_to_event_then_update(
            request1,
            TimeResponse::DurationElapsed {
                id: model.debounce_time_id.unwrap(),
            },
            &mut model,
        )
        .assert_empty();

        // resolving the first debounce should not set the debounce_complete flag
        assert!(!model.debounce_complete);

        // resolve and update
        app.resolve_to_event_then_update(
            request2,
            TimeResponse::DurationElapsed {
                id: model.debounce_time_id.unwrap(),
            },
            &mut model,
        )
        .assert_empty();

        // resolving the second debounce should set the debounce_complete flag
        assert!(model.debounce_complete);
    }

    #[test]
    pub fn test_start_debounce_then_clear() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();
        let mut debounce = app
            .update(Event::StartDebounce, &mut model)
            .expect_one_effect()
            .expect_time();
        let timer_id = model.debounce_time_id.unwrap();
        let _cancel = app
            .update(Event::Cancel(timer_id), &mut model)
            .expect_one_effect()
            .expect_time();
        // this is a little strange-looking. We have cleared the timer,
        // so the in-flight debounce should have resolved. But to force that
        // to happen, we have to run the app, and the easiest way to do that
        // is to resolve the original debounce effect with a fake outcome -
        // which will be ignored in favour of TimeResponse::Cleared
        let ev = app
            .resolve(
                &mut debounce,
                TimeResponse::DurationElapsed { id: timer_id },
            )
            .unwrap()
            .expect_one_event();
        let Event::DurationElapsed(_, TimeResponse::Cleared { id }) = ev else {
            panic!()
        };
        assert_eq!(id, timer_id);
    }
}
