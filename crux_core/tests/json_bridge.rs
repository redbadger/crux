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
        /// Like `Get`, but settles without emitting a follow-up effect, so the
        /// request's registry slot is left free rather than immediately taken
        /// by the render that `Get` would cause.
        GetQuietly,
        Settle,
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
                Event::GetQuietly => Http::get("http://example.com/")
                    .build()
                    .then_send(|_| Event::Settle),
                Event::Settle => Command::done(),
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
    use crate::app::EffectFfi;

    use super::core::Bridge;
    use crux_core::{
        Core,
        bridge::{EffectId, Request},
    };
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
            panic!("Expected id to be a number, got: {:?}", request["id"])
        };
        assert_eq!(id.as_u64().unwrap(), 0);

        let Value::Object(effect) = &request["effect"] else {
            panic!(
                "Expected effect to be an object, got: {:?}",
                request["effect"]
            )
        };

        let Value::Null = &effect["Render"] else {
            panic!("Expected effect to be a 'Render' variant, got: {effect:?}")
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
            "could not deserialize event: unknown variant `Nopes`, expected one of `Trigger`, `Get`, `GetQuietly`, `Settle` at line 1 column 7"
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

    /// A fire-and-forget request has no continuation to store, so its id is
    /// simply not outstanding — resolving it is the same "not found" as
    /// resolving an id that was never issued.
    #[test]
    fn resolve_fire_and_forget() {
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
            "could not process response: Request with id 0 not found."
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

    /// An id identifies one request for the lifetime of the bridge, and is never
    /// handed out again once that request has been resolved.
    ///
    /// Ids used to be slab indices, which the slab reused the moment a request
    /// completed. A shell that resolved an id twice — a retry, a race, a bug —
    /// would then resolve whichever *unrelated* request had inherited the slot,
    /// and if the two outputs happened to deserialize compatibly (two HTTP
    /// requests, say) it succeeded silently.
    #[test]
    fn ids_are_not_reused_after_a_request_resolves() {
        let bridge = Bridge::new(Core::default());

        let first = request_http(&bridge);
        resolve_http(&bridge, first.id).expect("first resolve should work");

        // The slot backing `first` is now free.
        let second = request_http(&bridge);

        assert_ne!(
            first.id, second.id,
            "a resolved request's id was handed out again"
        );
    }

    #[test]
    fn resolving_a_request_twice_is_an_error() {
        let bridge = Bridge::new(Core::default());

        let first = request_http(&bridge);
        resolve_http(&bridge, first.id).expect("first resolve should work");

        // A second request, which previously inherited `first`'s slot and so
        // received the duplicate resolve below.
        let _second = request_http(&bridge);

        let Err(error) = resolve_http(&bridge, first.id) else {
            panic!("expected resolving a completed request to fail");
        };

        assert_eq!(
            error.to_string(),
            format!(
                "could not process response: Request with id {} not found.",
                first.id.0
            )
        );
    }

    fn request_http(bridge: &Bridge) -> Request<EffectFfi> {
        let mut effects_bytes = vec![];
        bridge
            .update(b"\"GetQuietly\"", &mut effects_bytes)
            .expect("event should process");

        let mut effects: Vec<Request<EffectFfi>> =
            serde_json::from_slice(&effects_bytes).expect("to deserialise");

        effects.remove(0)
    }

    fn resolve_http(
        bridge: &Bridge,
        id: EffectId,
    ) -> Result<(), crux_core::bridge::BridgeError<crux_core::bridge::JsonFfiFormat>> {
        let response = HttpResult::Ok(HttpResponse::ok().body("hello").build());
        let response = serde_json::to_vec(&response).expect("to serialise");

        let mut effects_bytes = vec![];
        bridge.resolve(id, &response, &mut effects_bytes)
    }
}
