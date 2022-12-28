mod shared {

    use crux_http::Http;
    use crux_macros::Effect;
    use serde::{Deserialize, Serialize};
    use url::Url;

    #[derive(Default)]
    pub(crate) struct App;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub enum Event {
        Get,
        Set(String),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct Model {
        pub body: String,
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
                        .get_json(Url::parse("http://example.com").unwrap(), Event::Set);
                }
                Event::Set(body) => {
                    model.body = body;
                }
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
    use crate::shared::{App, Effect, Event, Model};
    use crux_core::testing::AppTester;
    use crux_http::{HttpRequest, HttpResponse};

    #[test]
    fn with_tester() {
        let app = AppTester::<App, _>::default();
        let mut model = Model::default();

        let update = app.update(Event::Get, &mut model);

        assert_eq!(
            update.effects[0],
            Effect::Http(HttpRequest {
                method: "GET".to_string(),
                url: "http://example.com/".to_string()
            })
        );

        let update = update.effects[0].resolve(&HttpResponse {
            status: 200,
            body: serde_json::to_vec("hello").unwrap(),
        });

        let actual = update.events;
        let expected = vec![Event::Set("hello".to_string())];
        assert_eq!(actual, expected);
    }
}
