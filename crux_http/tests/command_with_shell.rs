mod shared {
    use crux_core::{
        macros::Effect,
        render::{render, Render},
        Command,
    };
    use crux_http::command::Http;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub(crate) struct App;

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        Get,
        GetJson,

        // events local to the core
        #[serde(skip)]
        Set(crux_http::Result<crux_http::Response<Vec<u8>>>),
        #[serde(skip)]
        SetJson(crux_http::Result<crux_http::Response<String>>),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct Model {
        pub status: u16,
        pub body: Vec<u8>,
        pub json_body: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct ViewModel {
        pub result: String,
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
            _caps: &Capabilities,
        ) -> Command<Effect, Event> {
            match event {
                Event::Get => Http::get("http://example.com")
                    .build()
                    .then_send(Event::Set),
                Event::GetJson => Http::get("http://example.com")
                    .expect_json::<String>()
                    .build()
                    .then_send(Event::SetJson),
                Event::Set(response) => {
                    let mut response = response.unwrap();
                    model.status = response.status().into();
                    model.body = response.take_body().unwrap();

                    render()
                }
                Event::SetJson(response) => {
                    model.json_body = response.unwrap().take_body().unwrap();

                    render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                result: format!(
                    "Status: {}, Body: {}, Json Body: {}",
                    model.status,
                    String::from_utf8_lossy(&model.body),
                    &model.json_body
                ),
            }
        }
    }

    #[derive(Effect)]
    #[allow(dead_code)]
    pub(crate) struct Capabilities {
        pub http: crux_http::Http<Event>,
        pub render: Render<Event>,
    }
}

mod shell {
    use super::shared::{App, Effect, Event};
    use anyhow::Result;
    use crux_core::Core;
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};
    use std::collections::VecDeque;

    enum Task {
        Event(Event),
        Effect(Effect),
    }

    pub(crate) fn run(core: &Core<App>, event: Event) -> Result<Vec<HttpRequest>> {
        let mut queue: VecDeque<Task> = VecDeque::new();

        queue.push_back(Task::Event(event));

        let mut received: Vec<HttpRequest> = vec![];

        while !queue.is_empty() {
            let task = queue.pop_front().expect("an event");

            match task {
                Task::Event(event) => {
                    enqueue_effects(&mut queue, core.process_event(event));
                }
                Task::Effect(effect) => match effect {
                    Effect::Render(_) => (),
                    Effect::Http(mut request) => {
                        let http_request = &request.operation;

                        received.push(http_request.clone());
                        let response = HttpResponse::ok().json("Hello").build();

                        enqueue_effects(
                            &mut queue,
                            core.resolve(&mut request, HttpResult::Ok(response))
                                .expect("effect should resolve"),
                        );
                    }
                },
            };
        }

        Ok(received)
    }

    fn enqueue_effects(queue: &mut VecDeque<Task>, effects: Vec<Effect>) {
        queue.append(&mut effects.into_iter().map(Task::Effect).collect())
    }
}

mod tests {
    use crate::{
        shared::{App, Event},
        shell::run,
    };
    use anyhow::Result;
    use crux_core::Core;
    use crux_http::protocol::HttpRequest;

    #[test]
    pub fn test_http() -> Result<()> {
        let core: Core<App> = Core::default();

        let received = run(&core, Event::Get)?;

        assert_eq!(
            received,
            vec![HttpRequest::get("http://example.com/").build()]
        );

        assert_eq!(
            core.view().result,
            "Status: 200, Body: \"Hello\", Json Body: "
        );
        Ok(())
    }

    #[test]
    pub fn test_http_json() -> Result<()> {
        let core: Core<App> = Core::default();

        let received = run(&core, Event::GetJson)?;

        assert_eq!(
            received,
            vec![HttpRequest::get("http://example.com/").build()]
        );
        assert_eq!(core.view().result, "Status: 0, Body: , Json Body: Hello");
        Ok(())
    }
}
