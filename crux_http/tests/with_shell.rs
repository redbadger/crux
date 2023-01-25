mod shared {
    use crux_core::render::Render;
    use crux_http::Http;
    use crux_macros::Effect;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub(crate) struct App;

    #[derive(Serialize, Deserialize)]
    pub enum Event {
        Get,
        GetJson,
        Set(crux_http::Result<crux_http::Response<Vec<u8>>>),
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

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {
            match event {
                Event::Get => {
                    caps.http.get("http://example.com").send(Event::Set);
                }
                Event::GetJson => {
                    caps.http
                        .get("http://example.com")
                        .expect_json::<String>()
                        .send(Event::SetJson);
                }
                Event::Set(response) => {
                    let mut response = response.unwrap();
                    model.status = response.status().into();
                    model.body = response.take_body().unwrap();
                    caps.render.render()
                }
                Event::SetJson(response) => {
                    model.json_body = response.unwrap().take_body().unwrap();
                    caps.render.render()
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
    pub(crate) struct Capabilities {
        pub http: Http<Event>,
        pub render: Render<Event>,
    }
}

mod shell {
    use super::shared::{App, Effect, Event, ViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_http::protocol::{HttpRequest, HttpResponse};
    use std::collections::VecDeque;

    pub enum Outcome {
        Http(HttpResponse),
    }

    enum CoreMessage {
        Event(Event),
        Response(Vec<u8>, Outcome),
    }

    pub fn run(event: Event) -> Result<(Vec<Effect>, ViewModel)> {
        let core: Core<Effect, App> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Event(event));

        let mut received = vec![];

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let reqs = match msg {
                Some(CoreMessage::Event(m)) => core.process_event(&bcs::to_bytes(&m)?),
                Some(CoreMessage::Response(uuid, output)) => core.handle_response(
                    &uuid,
                    &match output {
                        Outcome::Http(x) => bcs::to_bytes(&x)?,
                    },
                ),
                _ => vec![],
            };
            let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs)?;

            for Request { uuid, effect } in reqs {
                match effect {
                    Effect::Render(_) => received.push(effect.clone()),
                    Effect::Http(HttpRequest { .. }) => {
                        received.push(effect);
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::Http(HttpResponse {
                                status: 200,
                                body: "\"Hello\"".as_bytes().to_owned(),
                            }),
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
    use crate::{
        shared::{Effect, Event},
        shell::run,
    };
    use anyhow::Result;
    use crux_core::render::RenderOperation;
    use crux_http::protocol::HttpRequest;

    #[test]
    pub fn test_http() -> Result<()> {
        let (received, view) = run(Event::Get)?;
        assert_eq!(
            received,
            vec![
                Effect::Http(HttpRequest {
                    method: "GET".to_string(),
                    url: "http://example.com/".to_string()
                }),
                Effect::Render(RenderOperation)
            ]
        );
        assert_eq!(view.result, "Status: 200, Body: \"Hello\", Json Body: ");
        Ok(())
    }

    #[test]
    pub fn test_http_json() -> Result<()> {
        let (received, view) = run(Event::GetJson)?;
        assert_eq!(
            received,
            vec![
                Effect::Http(HttpRequest {
                    method: "GET".to_string(),
                    url: "http://example.com/".to_string()
                }),
                Effect::Render(RenderOperation)
            ]
        );
        assert_eq!(view.result, "Status: 0, Body: , Json Body: Hello");
        Ok(())
    }
}
