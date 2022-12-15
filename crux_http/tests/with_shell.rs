mod shared {
    use crux_core::{capability::CapabilityContext, render::Render, App, WithContext};
    use crux_http::{Http, HttpRequest, HttpResponse};
    use serde::{Deserialize, Serialize};
    use url::Url;

    #[derive(Default)]
    pub(crate) struct MyApp;

    #[derive(Serialize, Deserialize)]
    pub enum MyEvent {
        Get,
        GetJson,
        Set(HttpResponse),
        SetJson(String),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct MyModel {
        pub status: u16,
        pub body: Vec<u8>,
        pub json_body: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct MyViewModel {
        pub result: String,
    }

    impl App for MyApp {
        type Event = MyEvent;
        type Model = MyModel;
        type ViewModel = MyViewModel;

        type Capabilities = MyCapabilities;

        fn update(&self, event: MyEvent, model: &mut MyModel, caps: &MyCapabilities) {
            match event {
                MyEvent::Get => {
                    caps.http
                        .get(Url::parse("http://example.com").unwrap(), MyEvent::Set);
                }
                MyEvent::GetJson => {
                    caps.http
                        .get_json(Url::parse("http://example.com").unwrap(), MyEvent::SetJson);
                }
                MyEvent::Set(response) => {
                    model.status = response.status;
                    model.body = response.body;
                    caps.render.render()
                }
                MyEvent::SetJson(body) => {
                    model.json_body = body;
                    caps.render.render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            MyViewModel {
                result: format!(
                    "Status: {}, Body: {}, Json Body: {}",
                    model.status,
                    String::from_utf8_lossy(&model.body),
                    &model.json_body
                ),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum MyEffect {
        Http(HttpRequest),
        Render,
    }

    pub(crate) struct MyCapabilities {
        pub http: Http<MyEvent>,
        pub render: Render<MyEvent>,
    }

    impl WithContext<MyApp, MyEffect> for MyCapabilities {
        fn new_with_context(context: CapabilityContext<MyEffect, MyEvent>) -> MyCapabilities {
            MyCapabilities {
                http: Http::new(context.with_effect(MyEffect::Http)),
                render: Render::new(context.with_effect(|_| MyEffect::Render)),
            }
        }
    }
}

mod shell {
    use super::shared::{MyApp, MyEffect, MyEvent, MyViewModel};
    use anyhow::Result;
    use crux_core::{Core, Request};
    use crux_http::{HttpRequest, HttpResponse};
    use std::collections::VecDeque;

    pub enum Outcome {
        Http(HttpResponse),
    }

    enum CoreMessage {
        Message(MyEvent),
        Response(Vec<u8>, Outcome),
    }

    pub fn run(event: MyEvent) -> Result<(Vec<MyEffect>, MyViewModel)> {
        let core: Core<MyEffect, MyApp> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(event));

        let mut received = vec![];

        while !queue.is_empty() {
            let msg = queue.pop_front();

            let reqs = match msg {
                Some(CoreMessage::Message(m)) => core.message(&bcs::to_bytes(&m)?),
                Some(CoreMessage::Response(uuid, output)) => core.response(
                    &uuid,
                    &match output {
                        Outcome::Http(x) => bcs::to_bytes(&x)?,
                    },
                ),
                _ => vec![],
            };
            let reqs: Vec<Request<MyEffect>> = bcs::from_bytes(&reqs)?;

            for Request { uuid, effect } in reqs {
                match effect {
                    MyEffect::Render => received.push(effect.clone()),
                    MyEffect::Http(HttpRequest { .. }) => {
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

        let view = bcs::from_bytes::<MyViewModel>(&core.view())?;
        Ok((received, view))
    }
}

mod tests {
    use crate::{
        shared::{MyEffect, MyEvent},
        shell::run,
    };
    use anyhow::Result;
    use crux_http::HttpRequest;

    #[test]
    pub fn test_http() -> Result<()> {
        let (received, view) = run(MyEvent::Get)?;
        assert_eq!(
            received,
            vec![
                MyEffect::Http(HttpRequest {
                    method: "GET".to_string(),
                    url: "http://example.com/".to_string()
                }),
                MyEffect::Render
            ]
        );
        assert_eq!(view.result, "Status: 200, Body: \"Hello\", Json Body: ");
        Ok(())
    }

    #[test]
    pub fn test_http_json() -> Result<()> {
        let (received, view) = run(MyEvent::GetJson)?;
        assert_eq!(
            received,
            vec![
                MyEffect::Http(HttpRequest {
                    method: "GET".to_string(),
                    url: "http://example.com/".to_string()
                }),
                MyEffect::Render
            ]
        );
        assert_eq!(view.result, "Status: 0, Body: , Json Body: Hello");
        Ok(())
    }
}
