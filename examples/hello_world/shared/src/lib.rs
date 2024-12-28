uniffi::setup_scaffolding!();

pub mod app; // not exposed so you can remove this in your project

use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::{bridge::Bridge, Core, Request};

pub use app::*;

lazy_static! {
    static ref CORE: Bridge<Effect, Hello> = Bridge::new(Core::new());
}

#[uniffi::export]
#[wasm_bindgen]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE.process_event(data)
}

#[uniffi::export]
#[wasm_bindgen]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    CORE.handle_response(id, data)
}

#[uniffi::export]
#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view()
}
