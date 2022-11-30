pub use app::*;
pub use crux_core::http;
pub use crux_core::key_value;
pub use crux_core::platform;
pub use crux_core::time;
use crux_core::Core;
pub use crux_core::Request;
use effect::Capabilities;
pub use effect::Effect;
use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod app;
pub mod effect;

// TODO hide this plumbing

uniffi_macros::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Core<Effect, Capabilities, CatFacts<Effect, Capabilities>> = Core::new();
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
