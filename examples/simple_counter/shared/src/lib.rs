pub mod app;

use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::{bridge::Bridge, Core, Request};

pub use app::*;

// TODO hide this plumbing

uniffi::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Bridge<Counter> = Bridge::new(Core::new());
}

#[wasm_bindgen]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE.process_event(data)
}

#[wasm_bindgen]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    CORE.handle_response(id, data)
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view()
}
