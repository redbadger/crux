mod shared {
    use crux_http::Http;
    use crux_macros::Effect;
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
        fn update(&self, _event: Event, _model: &mut Self::Model, _caps: &Capabilities) {}
        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            todo!();
        }
    }

    #[derive(Effect)]
    pub struct Capabilities {
        pub http: Http<Event>,
    }
}
mod test {
    use super::shared::{App, Capabilities, EffectFfi, Event, ViewModel};
    use crux_core::{
        bridge::Request,
        capability::Operation,
        typegen::{Export, TypeGen},
        Capability,
    };
    use crux_http::{
        protocol::{HttpRequest, HttpResponse},
        Http,
    };
    use std::env::temp_dir;
    use uuid::Uuid;

    // FIXME this test is quite slow
    #[test]
    fn generate_types() {
        let mut gen = TypeGen::new();

        gen.register_type::<Request<EffectFfi>>().unwrap();

        gen.register_type::<EffectFfi>().unwrap();
        gen.register_type::<HttpRequest>().unwrap();

        let sample_events = vec![Event::SendUuid(Uuid::new_v4())];
        gen.register_type_with_samples(sample_events).unwrap();

        gen.register_type::<HttpResponse>().unwrap();

        gen.register_type::<ViewModel>().unwrap();

        let output_root = temp_dir().join("crux_core_typegen_test");

        gen.swift("shared_types", output_root.join("swift"))
            .expect("swift type gen failed");

        gen.java("com.example.counter.shared_types", output_root.join("java"))
            .expect("java type gen failed");

        gen.typescript("shared_types", output_root.join("typescript"))
            .expect("typescript type gen failed");
    }

    impl Export for Capabilities {
        fn register_types(generator: &mut TypeGen) -> anyhow::Result<()> {
            // for each capability
            generator.register_type::<<Http<Event> as Capability<Event>>::Operation>()?;
            generator
                .register_type::<<<Http<Event> as Capability<Event>>::Operation as Operation>::Output>()?;

            Ok(())
        }
    }

    #[test]
    fn test_autodiscovery() {
        let mut gen = TypeGen::new();

        gen.register_type_with_samples(vec![Event::SendUuid(Uuid::new_v4())])
            .unwrap();

        gen.register_app::<App>()
            .expect("Should register types in App");

        let registry = match gen.state {
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

        assert!(registry.contains_key("HttpRequest"));
        assert!(registry.contains_key("HttpResponse"));
    }
}
