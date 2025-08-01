#![allow(clippy::missing_panics_doc)]

pub mod app;
pub mod config;
pub mod favorites;
pub mod location;
pub mod weather;

use std::sync::LazyLock;

pub use crux_core::{bridge::Bridge, Core, Request};

pub use app::*;
pub use location::model::*;
// TODO hide this plumbing

uniffi::include_scaffolding!("shared");

static CORE: LazyLock<Bridge<App>> = LazyLock::new(|| Bridge::new(Core::new()));

#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[must_use]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    match CORE.process_event(data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[must_use]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    match CORE.handle_response(id, data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[must_use]
pub fn view() -> Vec<u8> {
    match CORE.view() {
        Ok(view) => view,
        Err(e) => panic!("{e}"),
    }
}
