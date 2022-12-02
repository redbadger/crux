#[cfg(nope)]
mod shared {
    use crux_core::{render::Render, App, Capabilities, Command, Commander};
    use crux_http::{Http, HttpRequest, HttpResponse};
    use serde::{Deserialize, Serialize};
    use std::marker::PhantomData;
    use url::Url;

    #[derive(Default)]
    pub struct MyApp<Ef, Caps> {
        _marker: PhantomData<fn() -> (Ef, Caps)>,
    }

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

    impl<Ef> App<Ef> for MyApp<Ef>
    where
        Ef: Serialize + Clone + Default,
        // Caps: Default + Capabilities<Http<Ef>> + Capabilities<Render<Ef>>,
    {
        type Event = MyEvent;
        type Model = MyModel;
        type ViewModel = MyViewModel;

        // TODO
        type Capabilities = ();

        fn update(
            &self,
            event: MyEvent,
            model: &mut MyModel,
            caps: &Caps,
            commander: &Commander<Command<Ef, Self::Event>>,
        ) {
            let http: &Http<_> = caps.get();
            let render: &Render<_> = caps.get();

            match event {
                MyEvent::HttpGet => commander.send_commands(vec![
                    http.get(Url::parse("http://example.com").unwrap(), MyEvent::HttpSet)
                ]),
                MyEvent::HttpSet(response) => {
                    model.status = response.status;
                    model.body = response.body;
                    commander.send_command(render.render())
                }
            }
        }

        fn view(
            &self,
            model: &<Self as App<Ef, Caps>>::Model,
        ) -> <Self as App<Ef, Caps>>::ViewModel {
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
        pub http: Http<MyEffect>,
        pub render: Render<MyEffect>,
    }

    impl crux_core::Capabilities<Http<MyEffect>> for MyCapabilities {
        fn get(&self) -> &Http<MyEffect> {
            &self.http
        }
    }

    impl crux_core::Capabilities<Render<MyEffect>> for MyCapabilities {
        fn get(&self) -> &Render<MyEffect> {
            &self.render
        }
    }

    impl Default for MyCapabilities {
        fn default() -> Self {
            Self {
                http: Http::new(MyEffect::Http),
                render: Render::new(MyEffect::Render),
            }
        }
    }
}

#[cfg(nope)]
mod shell {
    use super::shared::{MyApp, MyCapabilities, MyEffect, MyEvent, MyViewModel};
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
        let core: Core<MyEffect, MyCapabilities, MyApp<MyEffect, MyCapabilities>> = Core::new();
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

#[cfg(nope)]
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
