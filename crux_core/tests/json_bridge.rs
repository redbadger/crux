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

    #[effect(facet_typegen)]
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
        Core, RequestKind,
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

    /// Every fire-and-forget request is issued the reserved id `0`, and
    /// nothing is stored for one — so the bridge can say what a shell
    /// resolving it actually did, rather than reporting a lookup miss.
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

        assert_eq!(render.id, EffectId::NOTIFICATION);

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

    /// An id says which effect it belongs to, so a resolve for the wrong one
    /// is rejected before the bytes are even looked at.
    #[test]
    fn resolve_with_the_wrong_effect_variant() {
        let bridge = Bridge::new(Core::default());

        let http = request_http(&bridge);
        assert_eq!(http.id.effect_index(), 0, "Http is the first variant");
        assert_eq!(http.id.kind(), RequestKind::Request);

        // The same request, relabelled as the `Render` variant.
        let mangled = EffectId(http.id.0 | (1 << 24));

        let Err(error) = resolve_http(&bridge, mangled) else {
            panic!("expected resolving under the wrong effect to fail");
        };

        assert_eq!(
            error.to_string(),
            format!(
                "could not process response: Request id {} names effect variant 1, but request {} was issued for effect variant 0.",
                mangled.0,
                http.id.sequence()
            )
        );
    }

    /// So is a resolve claiming a kind the request was not issued with.
    #[test]
    fn resolve_with_the_wrong_kind_bit() {
        let bridge = Bridge::new(Core::default());

        let http = request_http(&bridge);

        // The same request, relabelled as a stream.
        let mangled = EffectId(http.id.0 | (1 << 23));
        assert_eq!(mangled.kind(), RequestKind::Stream);

        let Err(error) = resolve_http(&bridge, mangled) else {
            panic!("expected resolving with the wrong kind to fail");
        };

        assert_eq!(
            error.to_string(),
            format!(
                "could not process response: Request id {} is marked as a Stream request, but request {} was issued as a Request.",
                mangled.0,
                http.id.sequence()
            )
        );
    }

    /// An effect index the enum does not have cannot be one the bridge issued.
    #[test]
    fn resolve_with_an_effect_variant_that_does_not_exist() {
        let bridge = Bridge::new(Core::default());

        let http = request_http(&bridge);
        let mangled = EffectId(http.id.0 | (9 << 24));

        let Err(error) = resolve_http(&bridge, mangled) else {
            panic!("expected resolving an unknown effect variant to fail");
        };

        assert_eq!(
            error.to_string(),
            format!(
                "could not process response: Request id {} names effect variant 9, but the effect has only 2 variants.",
                mangled.0
            )
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
