use http::Http;
use serde::{Deserialize, Serialize};

use crate::{Command, Request};

// The future version of the app trait
pub trait App: Default {
    type Event: Send + 'static;
    type Model: Default;
    type ViewModel: Serialize;
    type Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event>;

    fn view(&self, model: &Self::Model) -> Self::ViewModel;
}

// A faux HTTP capability, because using the real one would cause a circular crate reference
mod http {
    use std::future::Future;

    use serde::{Deserialize, Serialize};

    use crate::{capability::Operation, command::builder::RequestBuilder, Command};

    pub struct Http;

    #[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
    pub struct Request {
        pub method: String,
        pub url: String,
        pub body: Option<String>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Response {
        pub status: usize,
        pub body: String,
    }

    impl Operation for Request {
        type Output = Response;
    }

    impl Http {
        pub fn get<Effect, Event>(
            url: impl Into<String>,
        ) -> RequestBuilder<Effect, Event, impl Future<Output = Response>>
        where
            Effect: From<crate::Request<Request>> + Send + 'static,
            Event: Send + 'static,
        {
            let request = Request {
                method: "GET".to_string(),
                url: url.into(),
                body: None,
            };

            Command::request_from_shell(request)
        }

        pub fn post<Effect, Event>(
            url: impl Into<String>,
            body: impl Into<String>,
        ) -> RequestBuilder<Effect, Event, impl Future<Output = Response>>
        where
            Effect: From<crate::Request<Request>> + Send + 'static,
            Event: Send + 'static,
        {
            let body = body.into();
            let request = Request {
                method: "POST".to_string(),
                url: url.into(),
                body: if !body.is_empty() { Some(body) } else { None },
            };

            Command::request_from_shell(request)
        }
    }
}

#[derive(Default)]
struct Counter;

#[derive(Serialize, Deserialize)]
enum Event {
    Get,
    Increment,
    Decrement,

    #[serde(skip)]
    GotCount(http::Response),
}

enum Effect {
    Http(Request<http::Request>),
}

impl From<Request<http::Request>> for Effect {
    fn from(value: Request<http::Request>) -> Self {
        Effect::Http(value)
    }
}

impl App for Counter {
    type Event = Event;
    type Model = usize;
    type ViewModel = usize;
    type Effect = Effect;

    fn update(
        &self,
        event: Self::Event,
        model: &mut Self::Model,
    ) -> Command<Self::Effect, Self::Event> {
        match event {
            Event::Get => Http::get("http://example.com/counter").then_send(Event::GotCount),
            Event::Increment => Command::new(|ctx| async move {
                let _ = Http::post("http://example.com/counter/increment", "")
                    .into_future(ctx.clone())
                    .await;

                let response = Http::get("http://example.com/counter")
                    .into_future(ctx.clone())
                    .await;

                ctx.send_event(Event::GotCount(response));
            }),

            Event::Decrement => Http::post("http://example.com/counter/decrement", "")
                .then_request(|_response| Http::get("http://example.com/counter"))
                .then_send(Event::GotCount),

            Event::GotCount(response) => {
                if response.status == 200 {
                    if let Ok(count) = response.body.parse() {
                        *model = count;
                    }
                }

                Command::done()
            }
        }
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        *model
    }
}

#[test]
fn get_increment_and_decrement() {
    let app = Counter;
    let mut model = 0;

    let mut cmd = app.update(Event::Get, &mut model);

    let Effect::Http(mut request) = cmd.effects().next().unwrap();
    let http_request = &request.operation;

    assert_eq!(http_request.method, "GET");
    assert_eq!(http_request.url, "http://example.com/counter");

    request
        .resolve(http::Response {
            status: 200,
            body: "10".to_string(),
        })
        .expect("to resolve");

    let event = cmd.events().next().unwrap();

    let mut cmd = app.update(event, &mut model);

    assert!(cmd.is_done());
    assert_eq!(model, 10);

    assert_eq!(app.view(&model), 10);

    let mut cmd = app.update(Event::Increment, &mut model);

    let Effect::Http(mut request) = cmd.effects().next().unwrap();
    let http_request = &request.operation;

    assert_eq!(http_request.method, "POST");
    assert_eq!(http_request.url, "http://example.com/counter/increment");

    request
        .resolve(http::Response {
            status: 204,
            body: "".to_string(),
        })
        .expect("to resolve");

    let Effect::Http(mut request) = cmd.effects().next().unwrap();
    let http_request = &request.operation;

    assert_eq!(http_request.method, "GET");
    assert_eq!(http_request.url, "http://example.com/counter");

    request
        .resolve(http::Response {
            status: 200,
            body: "11".to_string(),
        })
        .expect("to resolve");

    let event = cmd.events().next().unwrap();

    let mut cmd = app.update(event, &mut model);

    assert!(cmd.is_done());
    assert_eq!(model, 11);

    assert_eq!(app.view(&model), 11);

    let mut cmd = app.update(Event::Decrement, &mut model);

    let Effect::Http(mut request) = cmd.effects().next().unwrap();
    let http_request = &request.operation;

    assert_eq!(http_request.method, "POST");
    assert_eq!(http_request.url, "http://example.com/counter/decrement");

    request
        .resolve(http::Response {
            status: 204,
            body: "".to_string(),
        })
        .expect("to resolve");

    let Effect::Http(mut request) = cmd.effects().next().unwrap();
    let http_request = &request.operation;

    assert_eq!(http_request.method, "GET");
    assert_eq!(http_request.url, "http://example.com/counter");

    request
        .resolve(http::Response {
            status: 200,
            body: "10".to_string(),
        })
        .expect("to resolve");

    let event = cmd.events().next().unwrap();

    let mut cmd = app.update(event, &mut model);

    assert!(cmd.is_done());
    assert_eq!(model, 10);

    assert_eq!(app.view(&model), 10);
}
