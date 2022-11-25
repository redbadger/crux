use crux_core::{http, key_value, platform, render, time};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Effect {
    Platform,
    Time,
    Http(http::Request),
    KeyValue(key_value::Request),
    Render,
}

pub(crate) struct Capabilities {
    pub platform: platform::Platform<Effect>,
    pub time: time::Time<Effect>,
    pub http: http::Http<Effect>,
    pub key_value: key_value::KeyValue<Effect>,
    pub render: render::Render<Effect>,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            platform: platform::Platform::new(Effect::Platform),
            time: time::Time::new(Effect::Time),
            http: http::Http::new(Effect::Http),
            key_value: key_value::KeyValue::new(Effect::KeyValue),
            render: render::Render::new(Effect::Render),
        }
    }
}
