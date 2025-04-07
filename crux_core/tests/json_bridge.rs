mod app {
    use crux_core::render::{render, Render};
    use crux_core::{macros::Effect, Command};
    use crux_http::command::Http;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize, Debug)]
    pub enum Event {
        Trigger,
        Get,
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
            event: Event,
            _model: &mut Self::Model,
            _caps: &Capabilities,
        ) -> Command<Effect, Event> {
            match event {
                Event::Trigger => render(),
                Event::Get => Http::get("http://example.com/")
                    .build()
                    .then_send(|_| Event::Trigger),
            }
        }

        fn view(&self, _model: &Self::Model) -> Self::ViewModel {
            unimplemented!();
        }
    }

    #[derive(Effect)]
    #[allow(dead_code)]
    pub struct Capabilities {
        pub http: crux_http::Http<Event>,
        pub render: Render<Event>,
    }
}

mod core {
    use crux_core::bridge::BridgeWithSerializer;

    use crate::app::App;

    pub type Bridge = BridgeWithSerializer<App>;
}

mod tests {

    use crate::app::EffectFfi;

    use super::core::Bridge;
    use crux_core::{bridge::Request, Core};
    use serde_json::{json, Deserializer, Value};

    #[test]
    fn event_effect_loop() {
        let bridge = Bridge::new(Core::default());
        let event = json!("Trigger");

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        bridge
            .process_event(&event, &mut result_ser)
            .expect("event should process");

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

    #[test]
    fn unknown_event() {
        // Unknown
        let bridge = Bridge::new(Core::default());
        let event = json!("Nopes");

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        let result = bridge.process_event(&event, &mut result_ser);

        let Err(error) = result else {
            panic!("Expected a DeserializeEvent error");
        };

        assert_eq!(
            error.to_string(),
            "could not deserialize event: unknown variant `Nopes`, expected `Trigger` or `Get`"
        )
    }

    #[test]
    fn bad_bytes_event() {
        // Unknown
        let bridge = Bridge::new(Core::default());
        let event: Vec<u8> = vec![1, 2, 3];
        let mut de = Deserializer::from_slice(&event);

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        let result = bridge.process_event(&mut de, &mut result_ser);

        let Err(error) = result else {
            panic!("Expected a DeserializeEvent error");
        };

        assert_eq!(
            error.to_string(),
            "could not deserialize event: expected value at line 1 column 1"
        )
    }

    #[test]
    fn resolve_error() {
        let bridge = Bridge::new(Core::default());
        let event = json!("Trigger");

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        bridge
            .process_event(&event, &mut result_ser)
            .expect("event should process");

        let mut effects: Vec<Request<EffectFfi>> =
            serde_json::from_slice(&effects_bytes).expect("to deserialise");

        let render = effects.remove(0);

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        let value = json!("Hi");

        // Render does not expect a value!
        let result = bridge.handle_response(render.id.0, value, &mut result_ser);

        let Err(error) = result else {
            panic!("expected an error");
        };

        assert_eq!(
            error.to_string(),
            "could not process response: Attempted to resolve a request that is not expected to be resolved."
        );
    }

    #[test]
    fn resolve_bad_value() {
        let bridge = Bridge::new(Core::default());
        let event = json!("Get");

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        bridge
            .process_event(&event, &mut result_ser)
            .expect("event should process");

        let mut effects: Vec<Request<EffectFfi>> =
            serde_json::from_slice(&effects_bytes).expect("to deserialise");

        let http = effects.remove(0);

        let mut effects_bytes = vec![];
        let mut result_ser = serde_json::Serializer::new(&mut effects_bytes);

        let event: Vec<u8> = vec![1, 2, 3];
        let mut de = Deserializer::from_slice(&event);

        // Resolve HTTP with a bad value
        let result = bridge.handle_response(http.id.0, &mut de, &mut result_ser);

        let Err(error) = result else {
            panic!("expected an error");
        };

        assert_eq!(
            error.to_string(),
            "could not deserialize provided effect output: expected value at line 1 column 1"
        );
    }
}
