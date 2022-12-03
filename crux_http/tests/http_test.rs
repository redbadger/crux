mod shared {
    use crux_core::{render::Render, App, CapabilitiesFactory, Command};
    use crux_http::{Http, HttpRequest, HttpResponse};
    use serde::{Deserialize, Serialize};
    use url::Url;

    #[derive(Default)]
    pub(crate) struct MyApp;

    #[derive(Serialize, Deserialize)]
    pub enum MyEvent {
        HttpGet,
        HttpSet(HttpResponse),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct MyModel {
        pub status: u16,
        pub body: Vec<u8>,
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
                MyEvent::HttpGet => {
                    caps.http
                        .get(Url::parse("http://example.com").unwrap(), MyEvent::HttpSet);
                }
                MyEvent::HttpSet(response) => {
                    model.status = response.status;
                    model.body = response.body;
                    caps.render.render()
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            MyViewModel {
                result: format!(
                    "Status: {}, Body: {}",
                    model.status,
                    String::from_utf8_lossy(&model.body)
                ),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum MyEffect {
        Http(HttpRequest),
        Render,
    }

    impl Default for MyEffect {
        fn default() -> Self {
            MyEffect::Render
        }
    }

    pub(crate) struct MyCapabilities {
        pub http: Http<MyEvent>,
        pub render: Render<MyEvent>,
    }

    impl CapabilitiesFactory<MyApp, MyEffect> for MyCapabilities {
        fn build(
            channel: crux_core::channels::Sender<Command<MyEffect, MyEvent>>,
        ) -> MyCapabilities {
            MyCapabilities {
                http: Http::new(channel.map_effect(MyEffect::Http)),
                render: Render::new(channel.map_effect(|_| MyEffect::Render)),
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

    pub fn run() -> Result<(Vec<MyEffect>, MyViewModel)> {
        let core: Core<MyEffect, MyApp> = Core::default();
        let mut queue: VecDeque<CoreMessage> = VecDeque::new();

        queue.push_back(CoreMessage::Message(MyEvent::HttpGet));

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

            for req in reqs {
                let Request { uuid, effect } = req;
                match effect {
                    MyEffect::Render => received.push(effect.clone()),
                    MyEffect::Http(HttpRequest { .. }) => {
                        received.push(effect);
                        queue.push_back(CoreMessage::Response(
                            uuid,
                            Outcome::Http(HttpResponse {
                                status: 200,
                                body: "Hello".as_bytes().to_owned(),
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
    use crate::{shared::MyEffect, shell::run};
    use anyhow::Result;
    use crux_http::HttpRequest;

    #[test]
    pub fn test_http() -> Result<()> {
        let (received, view) = run()?;
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
        assert_eq!(view.result, "Status: 200, Body: Hello");
        Ok(())
    }
}
