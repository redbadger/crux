mod app;
mod capabilities;

use std::sync::LazyLock;

pub use app::*;
pub use capabilities::delay::DelayOperation;
pub use crux_core::Request;
use crux_core::{
    bridge::{Bridge, EffectId},
    Core,
};

#[cfg(not(target_family = "wasm"))]
uniffi::include_scaffolding!("shared");

static CORE: LazyLock<Bridge<App>> = LazyLock::new(|| Bridge::new(Core::new()));

/// Ask the core to process an event
/// # Panics
/// If the core fails to process the event
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[must_use]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    let mut effects = Vec::new();
    match CORE.update(data, &mut effects) {
        Ok(()) => effects,
        Err(e) => panic!("{e}"),
    }
}

/// Ask the core to handle a response
/// # Panics
/// If the core fails to handle the response
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[must_use]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    let mut effects = Vec::new();
    match CORE.resolve(EffectId(id), data, &mut effects) {
        Ok(()) => effects,
        Err(e) => panic!("{e}"),
    }
}

/// Ask the core to render the view
/// # Panics
/// If the view cannot be serialized
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
#[must_use]
pub fn view() -> Vec<u8> {
    let mut view_model = Vec::new();
    match CORE.view(&mut view_model) {
        Ok(()) => view_model,
        Err(e) => panic!("{e}"),
    }
}
