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

impl crux_core::Capabilities<platform::Platform<Effect>> for Capabilities {
    fn get(&self) -> &platform::Platform<Effect> {
        &self.platform
    }
}

impl crux_core::Capabilities<time::Time<Effect>> for Capabilities {
    fn get(&self) -> &time::Time<Effect> {
        &self.time
    }
}

impl crux_core::Capabilities<http::Http<Effect>> for Capabilities {
    fn get(&self) -> &http::Http<Effect> {
        &self.http
    }
}

impl crux_core::Capabilities<key_value::KeyValue<Effect>> for Capabilities {
    fn get(&self) -> &key_value::KeyValue<Effect> {
        &self.key_value
    }
}

impl crux_core::Capabilities<render::Render<Effect>> for Capabilities {
    fn get(&self) -> &render::Render<Effect> {
        &self.render
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
