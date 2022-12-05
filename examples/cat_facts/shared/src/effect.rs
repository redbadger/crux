use crux_core::render::Render;
use crux_http::{Http, HttpRequest};
use crux_kv::{KeyValue, KeyValueRequest};
use crux_macros::Capabilities;
use crux_platform::Platform;
use crux_time::Time;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Effect {
    Http(HttpRequest),
    KeyValue(KeyValueRequest),
    Platform,
    Render,
    Time,
}

#[derive(Capabilities)]
pub(crate) struct Capabilities {
    pub http: Http<Effect>,
    pub key_value: KeyValue<Effect>,
    pub platform: Platform<Effect>,
    pub render: Render<Effect>,
    pub time: Time<Effect>,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            http: Http::new(Effect::Http),
            key_value: KeyValue::new(Effect::KeyValue),
            platform: Platform::new(Effect::Platform),
            render: Render::new(Effect::Render),
            time: Time::new(Effect::Time),
        }
    }
}
