mod shared {

    use std::{cmp::max, collections::HashMap};

    use crux_core::{macros::Effect, Command};
    use crux_http::command::Http;
    use futures_util::join;
    use http_types::StatusCode;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub(crate) struct App;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub enum Event {
        Get,
        Post,
        PostForm,
        GetPostChain,
        ConcurrentGets,
        ComposeComplete(StatusCode),

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
        type Effect = Effect;

        fn update(
            &self,
            event: Event,
            model: &mut Model,
            _caps: &Capabilities,
        ) -> Command<Effect, Event> {
            match event {
                Event::Get => Http::get("http://example.com")
                    .header("Authorization", "secret-token")
                    .expect_string()
                    .build()
                    .then_send(Event::Set),
                Event::Post => Http::post("http://example.com")
                    .body_bytes("The Body".as_bytes())
                    .expect_string()
                    .build()
                    .then_send(Event::Set),
                Event::PostForm => {
                    let form = HashMap::from([("key", "value")]);
                    Http::post("http://example.com")
                        .body_form(&form)
                        .expect("could not serialize form data")
                        .expect_string()
                        .build()
                        .then_send(Event::Set)
                }
                Event::GetPostChain => Command::new(|ctx| async move {
                    let mut response = Http::get("http://example.com")
                        .build()
                        .into_future(ctx.clone())
                        .await
                        .unwrap();
                    let text = response.body_string().expect("response should have body");

                    let response = Http::post(format!("http://example.com/{}", text))
                        .build()
                        .into_future(ctx.clone())
                        .await
                        .unwrap();

                    ctx.send_event(Event::ComposeComplete(response.status()))
                }),
                Event::ConcurrentGets => Command::new(|ctx| async move {
                    let one = Http::get("http://example.com/one")
                        .build()
                        .into_future(ctx.clone());
                    let two = Http::get("http://example.com/two")
                        .build()
                        .into_future(ctx.clone());

                    let (response_one, response_two) = join!(one, two);

                    let one = response_one.unwrap();
                    let two = response_two.unwrap();

                    let status =
                        StatusCode::try_from(max::<u16>(one.status().into(), two.status().into()))
                            .unwrap();

                    ctx.send_event(Event::ComposeComplete(status))
                }),
                Event::ComposeComplete(status) => {
                    model.values.push(status.to_string());
                    Command::done()
                }
                Event::Set(Ok(mut response)) => {
                    model.body = response.take_body().unwrap();
                    model.values = response
                        .header("my_header")
                        .unwrap()
                        .iter()
                        .map(|v| v.to_string())
                        .collect();
                    Command::done()
                }
                Event::Set(Err(_)) => Command::done(),
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            ViewModel {
                result: format!("Body: {}", model.body),
            }
        }
    }

    #[derive(Effect)]
    #[allow(unused)]
    pub(crate) struct Capabilities {
        pub http: crux_http::Http<Event>,
    }
}

mod tests {
    use assert_matches::assert_matches;

    use crate::shared::{App, Effect, Event, Model};
    use crux_core::testing::AppTester;
    use crux_http::protocol::{HttpRequest, HttpResponse, HttpResult};

    #[test]
    fn get() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request = &mut app
            .update(Event::Get, &mut model)
            .expect_one_effect()
            .expect_http();

        assert_eq!(
            request.operation,
            HttpRequest::get("http://example.com/")
                .header("authorization", "secret-token")
                .build()
        );

        let actual = app
            .resolve(
                request,
                HttpResult::Ok(
                    HttpResponse::ok()
                        .json("hello")
                        .header("my_header", "my_value1")
                        .header("my_header", "my_value2")
                        .build(),
                ),
            )
            .expect("Resolves successfully")
            .expect_one_event();

        assert_matches!(actual.clone(), Event::Set(Ok(response)) => {
            assert_eq!(response.body().unwrap(), "\"hello\"");
            assert_eq!(response.header("my_header").unwrap().iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>(), vec!["my_value1", "my_value2"]);
        });

        app.update(actual, &mut model).assert_empty();
        assert_eq!(model.body, "\"hello\"");
        assert_eq!(model.values, vec!["my_value1", "my_value2"]);
    }

    #[test]
    fn post() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request = &mut app
            .update(Event::Post, &mut model)
            .expect_one_effect()
            .expect_http();

        assert_eq!(
            request.operation,
            HttpRequest::post("http://example.com/")
                .header("content-type", "application/octet-stream")
                .body("The Body")
                .build()
        );

        let actual = app
            .resolve(
                request,
                HttpResult::Ok(HttpResponse::ok().json("The Body").build()),
            )
            .expect("Resolves successfully")
            .expect_one_event();

        assert_matches!(actual, Event::Set(Ok(response)) => {
            assert_eq!(response.body().unwrap(), "\"The Body\"");
        });
    }

    #[test]
    fn post_form() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request = &mut app
            .update(Event::PostForm, &mut model)
            .expect_one_effect()
            .expect_http();

        assert_eq!(
            request.operation,
            HttpRequest::post("http://example.com/")
                .header("content-type", "application/x-www-form-urlencoded")
                .body("key=value")
                .build()
        );

        let actual = app
            .resolve(
                request,
                HttpResult::Ok(HttpResponse::ok().json("key=value").build()),
            )
            .expect("Resolves successfully")
            .expect_one_event();

        assert_matches!(actual, Event::Set(Ok(response)) => {
            assert_eq!(response.body().unwrap(), "\"key=value\"");
        });
    }

    #[test]
    fn get_post_chain() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request = &mut app
            .update(Event::GetPostChain, &mut model)
            .expect_one_effect()
            .expect_http();

        assert_eq!(
            request.operation,
            HttpRequest::get("http://example.com/").build()
        );

        let request = &mut app
            .resolve(
                request,
                HttpResult::Ok(HttpResponse::ok().body("secret_place").build()),
            )
            .expect("Resolves successfully")
            .expect_one_effect()
            .expect_http();

        assert_eq!(
            request.operation,
            HttpRequest::post("http://example.com/secret_place").build()
        );

        let actual = app
            .resolve(request, HttpResult::Ok(HttpResponse::status(201).build()))
            .expect("Resolves successfully")
            .expect_one_event();

        assert_matches!(actual, Event::ComposeComplete(status) => {
            assert_eq!(status, 201);
        });
    }

    #[test]
    fn concurrent_gets() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let mut requests = app
            .update(Event::ConcurrentGets, &mut model)
            .take_effects(Effect::is_http);

        let request_one = &mut requests.pop_front().unwrap().expect_http();

        assert_eq!(
            request_one.operation,
            HttpRequest::get("http://example.com/one").build()
        );

        let request_two = &mut requests.pop_front().unwrap().expect_http();

        assert_eq!(
            request_two.operation,
            HttpRequest::get("http://example.com/two").build()
        );

        // Resolve second request first, should not matter
        let update = app
            .resolve(
                request_two,
                HttpResult::Ok(HttpResponse::ok().body("one").build()),
            )
            .expect("Resolves successfully");

        // Nothing happens yet
        assert!(update.effects.is_empty());
        assert!(update.events.is_empty());

        let actual = app
            .resolve(
                request_one,
                HttpResult::Ok(HttpResponse::ok().body("one").build()),
            )
            .expect("Resolves successfully")
            .expect_one_event();

        assert_matches!(actual, Event::ComposeComplete(status) => {
            assert_eq!(status, 200);
        });
    }

    #[test]
    fn test_shell_error() {
        let app = AppTester::<App>::default();
        let mut model = Model::default();

        let request = &mut app
            .update(Event::Get, &mut model)
            .expect_one_effect()
            .expect_http();

        assert_eq!(
            request.operation,
            HttpRequest::get("http://example.com/")
                .header("authorization", "secret-token")
                .build()
        );

        let actual = app
            .resolve(
                request,
                HttpResult::Err(crux_http::HttpError::Io(
                    "Socket shenanigans prevented the request".to_string(),
                )),
            )
            .expect("Resolves successfully")
            .expect_one_event();

        let Event::Set(Err(crux_http::HttpError::Io(error))) = actual else {
            panic!("Expected original error back")
        };

        assert_eq!(error, "Socket shenanigans prevented the request")
    }
}
