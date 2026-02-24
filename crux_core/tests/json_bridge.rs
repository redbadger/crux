mod app {
    use crux_core::{
        Command,
        render::{RenderOperation, render},
    };
    use crux_http::{command::Http, protocol::HttpRequest};
    use crux_macros::effect;
    use serde::{Deserialize, Serialize};

    #[derive(Default)]
    pub struct App;

    #[derive(Serialize, Deserialize, Debug)]
    pub enum Event {
        Trigger,
        Get,
    }

    #[effect(typegen)]
    pub enum Effect {
        Http(HttpRequest),
        Render(RenderOperation),
    }

    #[derive(Serialize, Deserialize)]
    pub struct ViewModel;
    impl crux_core::App for App {
        type Event = Event;
        type Model = ();
        type ViewModel = ViewModel;
        type Effect = Effect;

        fn update(&self, event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
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
}

mod core {
    use crux_core::bridge::{Bridge as BridgeWithFormat, JsonFfiFormat};

    use crate::app::App;

    pub type Bridge = BridgeWithFormat<App, JsonFfiFormat>;
}

mod tests {
    use crate::app::{EffectFfi, Event};

    use super::core::Bridge;
    use crux_core::bridge::{MaybeSerialized, Response};
    use crux_core::{Core, bridge::Request};
    use crux_http::protocol::{HttpResponse, HttpResult};
    use serde_json::Value;

    #[test]
    fn event_effect_loop() {
        let bridge = Bridge::new(Core::default());
        let event = b"\"Trigger\"";

        let mut effects_bytes = vec![];

        bridge
            .update(event, &mut effects_bytes)
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
        let event = b"\"Nopes\"";

        let mut effects_bytes = vec![];

        let result = bridge.update(event, &mut effects_bytes);

        let Err(error) = result else {
            panic!("Expected a DeserializeEvent error");
        };

        assert_eq!(
            error.to_string(),
            "could not deserialize event: unknown variant `Nopes`, expected `Trigger` or `Get` at line 1 column 7"
        );
    }

    #[test]
    fn bad_bytes_event() {
        // Unknown
        let bridge = Bridge::new(Core::default());
        let event = b"123";

        let mut effects_bytes = vec![];

        let result = bridge.update(event, &mut effects_bytes);

        let Err(error) = result else {
            panic!("Expected a DeserializeEvent error");
        };

        assert_eq!(
            error.to_string(),
            "could not deserialize event: expected value at line 1 column 1"
        );
    }

    #[test]
    fn resolve_error() {
        let bridge = Bridge::new(Core::default());
        let event = b"\"Trigger\"";

        let mut effects_bytes = vec![];

        bridge
            .update(event, &mut effects_bytes)
            .expect("event should process");

        let mut effects: Vec<Request<EffectFfi>> =
            serde_json::from_slice(&effects_bytes).expect("to deserialise");

        let render = effects.remove(0);

        let mut effects_bytes = vec![];

        let value = b"\"Hi\"";

        // Render does not expect a value!
        let result = bridge.resolve(render.id, value, &mut effects_bytes);

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
        let event = b"\"Get\"";

        let mut effects_bytes = vec![];

        bridge
            .update(event, &mut effects_bytes)
            .expect("event should process");

        let mut effects: Vec<Request<EffectFfi>> =
            serde_json::from_slice(&effects_bytes).expect("to deserialise");

        let http = effects.remove(0);

        let mut effects_bytes = vec![];

        let event = b"123";

        // Resolve HTTP with a bad value
        let result = bridge.resolve(http.id, event, &mut effects_bytes);

        let Err(error) = result else {
            panic!("expected an error");
        };

        assert_eq!(
            error.to_string(),
            "could not deserialize provided effect output: expected value at line 1 column 1"
        );
    }

    #[test]
    fn event_effect_loop_typed() {
        let bridge = Bridge::new(Core::default());
        let event = Event::Trigger;

        let mut effects_bytes = vec![];

        bridge
            .update(MaybeSerialized::Value(event), &mut effects_bytes)
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
    fn resolve_typed() {
        let bridge = Bridge::new(Core::default());
        let event = b"\"Get\"";

        let mut effects_bytes = vec![];

        bridge
            .update(event, &mut effects_bytes)
            .expect("event should process");

        let mut effects: Vec<Request<EffectFfi>> =
            serde_json::from_slice(&effects_bytes).expect("to deserialise");

        let http = effects.remove(0);

        let mut effects_bytes = vec![];

        // Resolve a typed HTTP result
        let result = bridge.resolve(
            http.id,
            Response::Value(Box::new(HttpResult::Ok(HttpResponse::ok().build()))),
            &mut effects_bytes,
        );

        let Ok(()) = result else {
            panic!("expected success");
        };
    }
}
