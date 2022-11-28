use crux_core::{http, key_value, platform, render, time, GetCapabilityInstance};
use serde::{Deserialize, Serialize};

use crate::app::platform::Platform;

#[derive(Clone, Serialize, Deserialize)]
pub enum Effect {
    Platform,
    Time,
    Http(http::Request),
    KeyValue(key_value::Request),
    Render,
}

#[derive(Serialize, Deserialize)]
pub enum Outcome {
    Platform(platform::Response),
    Time(time::Response),
    Http(http::Response),
    KeyValue(key_value::Response),
}

// Will get generated?
pub(crate) struct Capabilities {
    pub platform: platform::Platform<Effect>,
    pub time: time::Time<Effect>,
    pub http: http::Http<Effect>,
    pub key_value: key_value::KeyValue<Effect>,
    pub render: render::Render<Effect>,
}

impl GetCapabilityInstance for Capabilities {
    type Capability = platform::Platform<Effect>;

    fn capability(&self) -> platform::Platform<Effect> {
        self.platform
    }
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
