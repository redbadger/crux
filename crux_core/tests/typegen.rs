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
    #[operation(request, output = GetResult)]
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

    /// `StoreError` is only reachable through `GetResult::Err`, and nothing
    /// names it: the registry walks the output's shape and finds it itself.
    #[test]
    fn nested_output_types_are_registered_without_being_named() {
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
