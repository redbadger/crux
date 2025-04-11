uniffi::setup_scaffolding!();

pub mod app;
pub mod capabilities;

use lazy_static::lazy_static;

pub use crux_core::bridge::{Bridge, Request};
pub use crux_core::{Core, ResolveError};
pub use crux_http as http;

pub use app::*;
pub use capabilities::sse;

lazy_static! {
    static ref CORE: Bridge<App> = Bridge::new(Core::new());
}

#[uniffi::export]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    match CORE.process_event(data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

#[uniffi::export]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    match CORE.handle_response(id, data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

#[uniffi::export]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn view() -> Vec<u8> {
    match CORE.view() {
        Ok(view) => view,
        Err(e) => panic!("{e}"),
    }
}
