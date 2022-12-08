mod shared {
    use crux_core::{capability::CapabilityContext, App, CapabilitiesFactory};
    use crux_http::{Http, HttpRequest};
    use serde::{Deserialize, Serialize};
    use url::Url;

    #[derive(Default)]
    pub(crate) struct MyApp;

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    pub enum MyEvent {
        Get,
        Set(String),
    }

    #[derive(Default, Serialize, Deserialize)]
    pub struct MyModel {
        pub body: String,
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
                        .get_json(Url::parse("http://example.com").unwrap(), MyEvent::Set);
                }
                MyEvent::Set(body) => {
                    model.body = body;
                }
            }
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            MyViewModel {
                result: format!("Body: {}", model.body),
            }
        }
    }

    #[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum MyEffect {
        Http(HttpRequest),
    }

    pub(crate) struct MyCapabilities {
        pub http: Http<MyEvent>,
    }

    impl CapabilitiesFactory<MyApp, MyEffect> for MyCapabilities {
        fn build(context: CapabilityContext<MyEffect, MyEvent>) -> MyCapabilities {
            MyCapabilities {
                http: Http::new(context.map_effect(MyEffect::Http)),
            }
        }
    }
}

mod tests {
    use crate::shared::{MyApp, MyEffect, MyEvent, MyModel};
    use crux_core::testing::AppTester;
    use crux_http::{HttpRequest, HttpResponse};

    #[test]
    fn with_tester() {
        let app = AppTester::<MyApp, _>::default();
        let mut model = MyModel::default();

        let update = app.update(MyEvent::Get, &mut model);

        assert_eq!(
            update.effects[0],
            MyEffect::Http(HttpRequest {
                method: "GET".to_string(),
                url: "http://example.com/".to_string()
            })
        );

        let update = update.effects[0].resolve(&HttpResponse {
            status: 200,
            body: serde_json::to_vec("hello").unwrap(),
        });

        let actual = update.events;
        let expected = vec![MyEvent::Set("hello".to_string())];
        assert_eq!(actual, expected);
    }
}
