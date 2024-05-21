//! Test for the testing APIs

use crux_core::testing::AppTester;

mod app {
    use crux_core::macros::Effect;
    use crux_core::App;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Hello,
    }

    #[derive(Effect)]
    pub struct Capabilities {
        render: crux_core::render::Render<Event>,
    }

    #[derive(Default)]
    pub struct MyApp;

    impl App for MyApp {
        type Event = Event;
        type Model = String;
        type ViewModel = String;
        type Capabilities = Capabilities;

        fn update(&self, _event: Self::Event, _model: &mut Self::Model, caps: &Self::Capabilities) {
            caps.render.render()
        }

        fn view(&self, model: &Self::Model) -> Self::ViewModel {
            model.clone()
        }
    }
}

#[test]
fn app_tester_new() {
    let app = app::MyApp;
    let tester = AppTester::new(app);

    let mut model = "Hello".to_string();

    let update = tester.update(app::Event::Hello, &mut model);

    let effects = update.into_effects();

    assert_eq!(effects.count(), 1);
}
