#[cfg(feature = "typegen")]
mod shared {
    use crux_core::Command;
    use crux_core::render::RenderOperation;
    use crux_macros::effect;
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
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            unimplemented!();
        }
    }

    #[effect(typegen)]
    pub enum Effect {
        Render(RenderOperation),
    }
}

#[cfg(feature = "typegen")]
mod test {
    use super::shared::{App, Event};
    use crux_core::type_generation::serde::TypeGen;
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
            crux_core::type_generation::serde::State::Registering(tracer, _) => {
                tracer.registry().expect("Should get registry")
            }
            crux_core::type_generation::serde::State::Generating(_) => {
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

#[cfg(feature = "facet_typegen")]
mod facet_shared {
    // `#[derive(Facet)]` generates `unsafe` methods.
    #![allow(clippy::unsafe_derive_deserialize)]

    use crux_core::{
        Command,
        macros::{Operation, effect},
        render::RenderOperation,
    };
    use facet::Facet;
    use serde::{Deserialize, Serialize};

    #[derive(Facet)]
    #[repr(C)]
    pub enum Event {
        None,
    }

    #[derive(Facet)]
    pub struct ViewModel;

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(request, output = GetResult, register(StoreError))]
    pub struct Get {
        pub key: String,
    }

    #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
    #[operation(notify)]
    pub struct Publish(pub Vec<u8>);

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[repr(C)]
    pub enum GetResult {
        Ok(Vec<u8>),
        Err(StoreError),
    }

    #[derive(Facet, Debug, Clone, Serialize, Deserialize)]
    #[repr(C)]
    pub enum StoreError {
        NotFound,
    }

    #[effect(facet_typegen)]
    pub enum Effect {
        Render(RenderOperation),
        Get(Get),
        Publish(Publish),
    }

    #[derive(Default)]
    pub struct App;

    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
            Command::done()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            ViewModel
        }
    }
}

#[cfg(feature = "facet_typegen")]
mod facet_test {
    use crux_core::{RequestKind, type_generation::facet::TypeRegistry};

    use super::facet_shared::App;

    #[test]
    fn effect_variants_carry_their_declared_request_kind() {
        let generator = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry");

        let kinds = generator
            .effect_kinds()
            .get("Effect")
            .expect("`Effect` should have recorded kinds");

        assert_eq!(
            kinds,
            &vec![
                ("Render".to_string(), Some(RequestKind::Notify)),
                ("Get".to_string(), Some(RequestKind::Request)),
                ("Publish".to_string(), Some(RequestKind::Notify)),
            ]
        );
    }

    #[test]
    fn registered_types_include_those_named_by_the_derive() {
        let registry = TypeRegistry::new()
            .register_app::<App>()
            .expect("should register the app")
            .build()
            .expect("should build the registry")
            .registry();

        for name in ["Get", "GetResult", "StoreError", "Publish", "Effect"] {
            assert!(
                registry.keys().any(|key| key.name == name),
                "expected {name} in the registry, got {:?}",
                registry.keys().collect::<Vec<_>>()
            );
        }
    }
}
