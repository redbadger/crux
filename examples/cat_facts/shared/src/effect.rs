use crux_core::{
    http::{Http, HttpRequest},
    key_value::{KeyValue, KeyValueRequest},
    render::Render,
    time::Time,
};
use crux_platform::Platform;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum Effect {
    Http(HttpRequest),
    KeyValue(KeyValueRequest),
    Platform,
    Render,
    Time,
}

impl Default for Effect {
    fn default() -> Self {
        Effect::Render
    }
}

// Will get generated?
pub(crate) struct Capabilities {
    pub http: Http<Effect>,
    pub key_value: KeyValue<Effect>,
    pub platform: Platform<Effect>,
    pub render: Render<Effect>,
    pub time: Time<Effect>,
}

impl crux_core::Capabilities<Http<Effect>> for Capabilities {
    fn get(&self) -> &Http<Effect> {
        &self.http
    }
}

impl crux_core::Capabilities<KeyValue<Effect>> for Capabilities {
    fn get(&self) -> &KeyValue<Effect> {
        &self.key_value
    }
}

impl crux_core::Capabilities<Platform<Effect>> for Capabilities {
    fn get(&self) -> &Platform<Effect> {
        &self.platform
    }
}

impl crux_core::Capabilities<Render<Effect>> for Capabilities {
    fn get(&self) -> &Render<Effect> {
        &self.render
    }
}

impl crux_core::Capabilities<Time<Effect>> for Capabilities {
    fn get(&self) -> &Time<Effect> {
        &self.time
    }
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
