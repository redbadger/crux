#[cfg(feature = "typegen")]
mod shared {
    use crux_core::Command;
    use crux_core::macros::{Effect, Export};
    use crux_core::render::Render;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize, Debug)]
    pub enum Event {
        None,
        SendUuid(uuid::Uuid),
    }

    #[derive(Serialize, Deserialize)]
    pub struct ViewModel;
    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;
        type Effect = Effect;

        fn update(
            &self,
            _event: Event,
            _model: &mut Self::Model,
            _caps: &Capabilities,
        ) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            unimplemented!();
        }
    }

    #[derive(Effect, Export)]
    pub struct Capabilities {
        #[allow(dead_code)]
        pub render: Render<Event>,
    }
}

#[cfg(feature = "typegen")]
mod test {
    use super::shared::{App, Event};
    use crux_core::typegen::TypeGen;
    use uuid::Uuid;

    // FIXME this test is quite slow
    #[test]
    fn generate_types() {
        let mut typegen = TypeGen::new();

        let sample_events = vec![Event::SendUuid(Uuid::new_v4())];
        typegen.register_type_with_samples(sample_events).unwrap();

        typegen.register_app::<App>().unwrap();

        let temp = assert_fs::TempDir::new().unwrap();
        let output_root = temp.join("crux_core_typegen_test");

        typegen
            .swift("SharedTypes", output_root.join("swift"))
            .expect("swift type gen failed");

        typegen
            .java("com.example.counter.shared_types", output_root.join("java"))
            .expect("java type gen failed");

        typegen
            .typescript("shared_types", output_root.join("typescript"))
            .expect("typescript type gen failed");
    }

    // TODO: instead of using the Render capability here, it would be better to also test against a custom
    // capability that has an output type
    #[test]
    fn test_autodiscovery() {
        let mut typegen = TypeGen::new();

        typegen
            .register_samples(vec![Event::SendUuid(Uuid::new_v4())])
            .unwrap();

        typegen
            .register_app::<App>()
            .expect("Should register types in App");

        let registry = match typegen.state {
            crux_core::typegen::State::Registering(tracer, _) => {
                tracer.registry().expect("Should get registry")
            }
            crux_core::typegen::State::Generating(_) => {
                panic!("Expected to still be in registering stage")
            }
        };

        dbg!(&registry);

        assert!(registry.contains_key("Event"));
        assert!(registry.contains_key("ViewModel"));

        assert!(registry.contains_key("Effect"));
        assert!(registry.contains_key("RenderOperation"));
    }
}
