pub mod app;
pub mod capabilities;

use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

use crux_core::bridge::Bridge;
pub use crux_core::{bridge::Request, Core};

pub use app::*;

uniffi::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Bridge<Effect, NoteEditor> = Bridge::new(Core::new::<Capabilities>());
}

#[wasm_bindgen]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE.process_event(data)
}

#[wasm_bindgen]
pub fn handle_response(uuid: &[u8], data: &[u8]) -> Vec<u8> {
    CORE.handle_response(uuid, data)
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view()
}
