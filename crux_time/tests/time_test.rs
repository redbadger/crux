#[cfg(feature = "chrono")]
mod shared {
    use std::time::SystemTime;

    use chrono::{DateTime, Utc};
    use crux_core::{
        Command,
        macros::effect,
        render::{RenderOperation, render},
    };
    use crux_time::{
        TimeRequest,
        command::{Time, TimerHandle, TimerOutcome},
    };
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Get,
        GetAsync,
        Set(SystemTime),
        StartDebounce,
        Cancel,

        #[serde(skip)]
        DurationElapsed(usize, TimerOutcome),
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
        pub debounce_timer_handle: Option<TimerHandle>,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct ViewModel {
        pub time: String,
    }

    impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, event: Event, model: &mut Model) -> Command<Effect, Event> {
            match event {
                Event::Get => Time::now().then_send(Event::Set),
                Event::GetAsync => Command::new(|ctx| async move {
                    ctx.send_event(Event::Set(Time::now().into_future(ctx.clone()).await));
                }),
                Event::Set(instant) => {
                    let time: DateTime<Utc> = instant.into();
                    model.time = time.to_rfc3339();

                    render()
                }
                Event::StartDebounce => {
                    let pending = model.debounce.start();

                    let (time, tid) = Time::notify_after(std::time::Duration::from_millis(300));
                    model.debounce_timer_handle = Some(tid);

                    time.then_send(event_with_user_info(pending, Event::DurationElapsed))
                }
                Event::DurationElapsed(pending, TimerOutcome::Completed(_)) => {
                    if model.debounce.resolve(pending) {
                        model.debounce_complete = true;
                    }

                    Command::done()
                }
                Event::DurationElapsed(_, TimerOutcome::Cleared) => Command::done(),
                Event::Cancel => {
                    if let Some(handle) = model.debounce_timer_handle.take() {
                        handle.clear();
                    }

                    Command::done()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                time: model.time.clone(),
            }
        }
    }

    #[effect]
    pub enum Effect {
        Time(TimeRequest),
        Render(RenderOperation),
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
    use crux_core::{Core, testing::AppTester};
    use crux_time::{TimeRequest, command::TimerOutcome, protocol::TimeResponse};

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
        let TimeRequest::NotifyAfter { id, .. } = request1.operation else {
            panic!("Expected NotifyAfter");
        };
        app.resolve_to_event_then_update(
            request1,
            TimeResponse::DurationElapsed { id },
            &mut model,
        )
        .assert_empty();

        // resolving the first debounce should not set the debounce_complete flag
        assert!(!model.debounce_complete);

        // resolve and update
        let TimeRequest::NotifyAfter { id, .. } = request2.operation else {
            panic!("Expected NotifyAfter");
        };

        app.resolve_to_event_then_update(
            request2,
            TimeResponse::DurationElapsed { id },
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
        let debounce = app
            .update(Event::StartDebounce, &mut model)
            .expect_one_effect()
            .expect_time();

        let TimeRequest::NotifyAfter { id: timer_id, .. } = debounce.operation else {
            panic!("Expected NotifyAfter");
        };

        let mut cancel = app
            .update(Event::Cancel, &mut model)
            .expect_one_effect()
            .expect_time();

        let ev = app
            .resolve(&mut cancel, TimeResponse::Cleared { id: timer_id })
            .unwrap()
            .expect_one_event();

        let Event::DurationElapsed(_, TimerOutcome::Cleared) = ev else {
            panic!()
        };
    }
}
