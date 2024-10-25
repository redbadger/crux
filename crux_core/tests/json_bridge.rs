mod app {
    use crux_core::macros::Effect;
    use crux_core::render::Render;
    use crux_core::Command;
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

        fn update(
            &self,
            _event: Event,
            _model: &mut Self::Model,
            caps: &Capabilities,
        ) -> Command<Self::Event> {
            caps.render.render()
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            unimplemented!();
        }
    }

    #[derive(Effect)]
    pub struct Capabilities {
        pub render: Render,
    }
}

mod core {
    use crux_core::bridge::BridgeWithSerializer;

    use crate::app::{App, Effect};

    pub type Bridge = BridgeWithSerializer<Effect, App>;
}

mod tests {

    use crate::core::Bridge;
    use crux_core::Core;
    use serde_json::{json, Value};

    #[test]
    fn event_effect_loop() {
        let bridge = Bridge::new(Core::default());
        let event = json!("Trigger");

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        bridge.process_event(&event, &mut result_ser);

        let actual_value: Value = serde_json::from_slice(&effects_bytes).unwrap();

        let Value::Array(effect_vals) = actual_value else {
            panic!("Expected an array of requests")
        };

        let Value::Object(request) = &effect_vals[0] else {
            panic!("Expected request to be an object")
        };

        let Value::Number(id) = &request["id"] else {
            panic!("Expected id to be a number, got: {:?}", &request["id"])
        };
        assert_eq!(id.as_u64().unwrap(), 0);

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
