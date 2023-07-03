mod shared {

    use crux_http::Http;
    use crux_macros::Effect;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub(crate) struct App;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub enum Event {
        Get,
        Post,

        // events local to the core
        Set(crux_http::Result<crux_http::Response<String>>),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct Model {
        pub body: String,
        pub values: Vec<String>,
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
                    caps.http
                        .get("http://example.com")
                        .header("Authorization", "secret-token")
                        .expect_string()
                        .send(Event::Set);
                }
                Event::Post => {
                    caps.http
                        .post("http://example.com")
                        .body_bytes("The Body".as_bytes())
                        .expect_string()
                        .send(Event::Set);
                }
                Event::Set(Ok(mut response)) => {
                    model.body = response.take_body().unwrap();
                    model.values = response
                        .header("my_header")
                        .unwrap()
                        .iter()
                        .map(|v| v.to_string())
                        .collect();
                }
                Event::Set(Err(_)) => {}
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                result: format!("Body: {}", model.body),
            }
        }
    }

    #[derive(Effect)]
    pub(crate) struct Capabilities {
        pub http: Http<Event>,
    }
}

mod tests {
    use assert_matches::assert_matches;

    use crate::shared::{App, Effect, Event, Model};
    use crux_core::testing::AppTester;
    use crux_http::protocol::{HttpHeader, HttpRequest, HttpResponse};

    #[test]
    fn get() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Get, &mut model);

        let Effect::Http(request) = update.effects_mut().next().unwrap();
        let http_request = &request.operation;

        assert_eq!(
            *http_request,
            HttpRequest {
                method: "GET".to_string(),
                url: "http://example.com/".to_string(),
                headers: vec![HttpHeader {
                    name: "authorization".to_string(),
                    value: "secret-token".to_string()
                }],
                ..Default::default()
            }
        );

        let update = app
            .resolve(
                request,
                HttpResponse {
                    status: 200,
                    body: serde_json::to_vec("hello").unwrap(),
                    headers: vec![
                        HttpHeader {
                            name: "my_header".to_string(),
                            value: "my_value1".to_string(),
                        },
                        HttpHeader {
                            name: "my_header".to_string(),
                            value: "my_value2".to_string(),
                        },
                    ],
                },
            )
            .expect("Resolves successfully");

        let actual = update.events.clone();
        assert_matches!(&actual[..], [Event::Set(Ok(response))] => {
            assert_eq!(*response.body().unwrap(), "\"hello\"".to_string());
            assert_eq!(*response.header("my_header").unwrap().iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>(), vec!["my_value1", "my_value2"]);
        });

        for event in update.events {
            app.update(event, &mut model);
        }
        assert_eq!(model.body, "\"hello\"");
        assert_eq!(model.values, vec!["my_value1", "my_value2"]);
    }

    #[test]
    fn post() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let mut update = app.update(Event::Post, &mut model);

        let Effect::Http(request) = update.effects_mut().next().unwrap();

        assert_eq!(
            request.operation,
            HttpRequest {
                method: "POST".to_string(),
                url: "http://example.com/".to_string(),
                headers: vec![HttpHeader {
                    name: "content-type".to_string(),
                    value: "application/octet-stream".to_string()
                }],
                body: "The Body".as_bytes().to_vec(),
            }
        );

        let update = app
            .resolve(
                request,
                HttpResponse {
                    status: 200,
                    body: serde_json::to_vec("The Body").unwrap(),
                    ..Default::default()
                },
            )
            .expect("Resolves successfully");

        let actual = update.events;
        assert_matches!(&actual[..], [Event::Set(Ok(response))] => {
            assert_eq!(*response.body().unwrap(), "\"The Body\"".to_string());
        });
    }
}
