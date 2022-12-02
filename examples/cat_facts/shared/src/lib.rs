pub use app::*;
use crux_core::Core;
pub use crux_core::Request;
pub use crux_http as http;
pub use crux_kv as key_value;
pub use crux_platform as platform;
pub use crux_time as time;
pub use effect::Effect;
use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::effect::CatFactCapabilities;

pub mod app;
pub mod effect;

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
