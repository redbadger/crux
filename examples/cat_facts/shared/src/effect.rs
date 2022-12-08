use crux_core::{capability::CapabilityContext, render::Render, Capability};
use crux_http::{Http, HttpRequest};
use crux_kv::{KeyValue, KeyValueOperation};
use crux_platform::Platform;
use crux_time::Time;
use serde::{Deserialize, Serialize};

use super::Event;
use crate::app::platform::PlatformCapabilities;

#[derive(Clone, Serialize, Deserialize)]
pub enum Effect {
    Http(HttpRequest),
    KeyValue(KeyValueOperation),
    Platform,
    Render,
    Time,
}

pub struct CatFactCapabilities {
    pub http: Http<Event>,
    pub key_value: KeyValue<Event>,
    pub platform: Platform<Event>,
    pub render: Render<Event>,
    pub time: Time<Event>,
}

impl crux_core::CapabilitiesFactory<super::CatFacts, Effect> for CatFactCapabilities {
    fn build(context: CapabilityContext<Effect, super::Event>) -> CatFactCapabilities {
        CatFactCapabilities {
            http: Http::new(context.map_effect(Effect::Http)),
            key_value: KeyValue::new(context.map_effect(Effect::KeyValue)),
            platform: Platform::new(context.map_effect(|_| Effect::Platform)),
            render: Render::new(context.map_effect(|_| Effect::Render)),
            time: Time::new(context.map_effect(|_| Effect::Time)),
        }
    }
}

impl From<&CatFactCapabilities> for PlatformCapabilities {
    fn from(incoming: &CatFactCapabilities) -> Self {
        PlatformCapabilities {
            platform: incoming.platform.map_event(super::Event::Platform),
            render: incoming.render.map_event(super::Event::Platform),
        }
    }
}
