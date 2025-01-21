//! Test for the testing APIs

use crux_core::testing::AppTester;

mod app {
    use crux_core::render::RenderOperation;
    use crux_core::{macros::Effect, Command};
    use crux_core::{render, App, Request};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub enum Event {
        Hello,
    }

    #[derive(Effect)]
    #[allow(dead_code)]
    pub struct Capabilities {
        render: crux_core::render::Render<Event>,
    }

    // FIXME: Remove after macro derive
    impl From<Request<RenderOperation>> for Effect {
        fn from(value: Request<RenderOperation>) -> Self {
            Self::Render(value)
        }
    }

    #[derive(Default)]
    pub struct MyApp;

    impl App for MyApp {
        type Event = Event;
        type Model = String;
        type ViewModel = String;
        type Capabilities = Capabilities;
        type Effect = Effect;

        fn update(
            &self,
            _event: Self::Event,
            _model: &mut Self::Model,
            _caps: &Self::Capabilities,
        ) -> Command<Effect, Event> {
            render::render()
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
