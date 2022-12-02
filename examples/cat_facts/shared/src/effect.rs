use crux_core::render::Render;
use crux_core::sender::SenderExt;
use crux_http::{Http, HttpRequest};
use crux_kv::{KeyValue, KeyValueRequest};
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
        sender: std::sync::mpsc::Sender<crux_core::Command<Effect, super::Event>>,
    ) -> CatFactCapabilities {
        // TODO: this is fucking hideous tbqh.  See if I can clean it up...
        CatFactCapabilities {
            http: Http::new(Box::new(sender.map_input(
                |command: crux_core::Command<HttpRequest, super::Event>| {
                    command.map_effect::<Effect, _>(Effect::Http)
                },
            ))),
        }
    }
}
