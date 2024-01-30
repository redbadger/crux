mod app {
    use crux_core::render::Render;
    use crux_macros::{Effect, Export};
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize, Debug)]
    pub enum Event {
        Trigger,
    }

    #[derive(Serialize, Deserialize)]
    pub struct ViewModel;
    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;

        fn update(&self, _event: Event, _model: &mut Self::Model, caps: &Capabilities) {
            caps.render.render();
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            unimplemented!();
        }
    }

    #[derive(Effect, Export)]
    pub struct Capabilities {
        pub render: Render<Event>,
    }
}

mod core {
    use crux_core::bridge::{BridgeWithSerializer, Serializer};
    use serde::Serialize;

    use crate::app::{App, Effect};

    #[derive(Clone)]
    pub struct Json;

    // Adapt serde_json for the Bridge
    impl Serializer for Json {
        type Error = serde_json::Error;

        fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, Self::Error> {
            serde_json::to_vec(&value)
        }

        fn deserialize<'de, T>(&self, data: &'de [u8]) -> Result<T, Self::Error>
        where
            T: serde::Deserialize<'de>,
        {
            serde_json::from_slice(data)
        }
    }

    pub type Bridge = BridgeWithSerializer<Effect, App, Json>;
}

mod tests {
    use crate::core::{Bridge, Json};
    use crux_core::Core;
    use serde_json::{json, Value};

    #[test]
    fn event_effect_loop() {
        let bridge = Bridge::new(Core::default(), Json);
        let event = json!("Trigger");

        let event_bytes = serde_json::to_vec(&event).unwrap();
        let effects_bytes = bridge.process_event(&event_bytes);

        let actual_value: Value = serde_json::from_slice(&effects_bytes).unwrap();

        let Value::Array(effect_vals) = actual_value else {
            panic!("Expected an array of requests")
        };

        let Value::Object(request) = &effect_vals[0] else {
            panic!("Expected request to be an object")
        };

        let Value::Array(uuid) = &request["uuid"] else {
            panic!("Expected uuid to be an array, got: {:?}", &request["uuid"])
        };
        assert_eq!(uuid.len(), 16);

        let Value::Object(effect) = &request["effect"] else {
            panic!(
                "Expected effect to be an object, got: {:?}",
                &request["effect"]
            )
        };

        let Value::Null = &effect["Render"] else {
            panic!(
                "Expected effect to be a 'Render' variant, got: {:?}",
                &effect
            )
        };
    }
}
