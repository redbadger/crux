pub mod app;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::Request;
use crux_core::{capability::CapabilityContext, render::Render, Core};
pub use crux_http as http;
pub use crux_kv as key_value;
pub use crux_platform as platform;
pub use crux_time as time;
use http::{Http, HttpRequest};
use key_value::{KeyValue, KeyValueOperation};
use platform::Platform;
use time::Time;

pub use app::*;

// TODO hide this plumbing

uniffi_macros::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Core<Effect, CatFacts> = Core::new::<CatFactCapabilities>();
}

#[wasm_bindgen]
pub fn message(data: &[u8]) -> Vec<u8> {
    CORE.message(data)
}

#[wasm_bindgen]
pub fn response(uuid: &[u8], data: &[u8]) -> Vec<u8> {
    CORE.response(uuid, data)
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view()
}

// TODO macro effect generation:
// crux_macros::generate_effect!(Effect, CatFactCapabilities);

#[derive(Clone, Serialize, Deserialize)]
pub enum Effect {
    Http(HttpRequest),
    KeyValue(KeyValueOperation),
    Platform,
    Render,
    Time,
}

impl crux_core::WithContext<CatFacts, Effect> for CatFactCapabilities {
    fn new_with_context(context: CapabilityContext<Effect, Event>) -> CatFactCapabilities {
        CatFactCapabilities {
            http: Http::new(context.with_effect(Effect::Http)),
            key_value: KeyValue::new(context.with_effect(Effect::KeyValue)),
            platform: Platform::new(context.with_effect(|_| Effect::Platform)),
            render: Render::new(context.with_effect(|_| Effect::Render)),
            time: Time::new(context.with_effect(|_| Effect::Time)),
        }
    }
}
