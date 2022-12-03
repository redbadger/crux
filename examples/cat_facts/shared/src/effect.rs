use std::sync::Arc;

use crux_core::{render::Render, Command};
use crux_http::{Http, HttpRequest};
use crux_kv::{KeyValue, KeyValueRequest};
use crux_platform::Platform;
use crux_time::Time;
use serde::{Deserialize, Serialize};

use crate::{app::platform::PlatformCapabilities, CatFact};

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

pub struct CatFactCapabilities {
    pub http: Http<super::Event>,
}

impl crux_core::CapabilityFactory<super::CatFacts, Effect> for CatFactCapabilities {
    fn build(
        sender: crux_core::channels::Sender<Command<Effect, super::Event>>,
    ) -> CatFactCapabilities {
        CatFactCapabilities {
            http: Http::new(sender.map_effect(Effect::Http)),
        }
    }
}

impl From<&CatFactCapabilities> for PlatformCapabilities {
    fn from(incoming: &CatFactCapabilities) -> Self {
        PlatformCapabilities {
            http: incoming.http.map_event(super::Event::Platform),
        }
    }
}
