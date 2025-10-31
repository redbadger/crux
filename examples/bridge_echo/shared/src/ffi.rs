use crux_core::{bridge::Bridge, Core};

use crate::App;

/// The main interface used by the shell
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
pub struct CoreFFI {
    core: Bridge<App>,
}

impl Default for CoreFFI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
impl CoreFFI {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    #[cfg_attr(
        feature = "wasm_bindgen",
        wasm_bindgen::prelude::wasm_bindgen(constructor)
    )]
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Bridge::new(Core::new()),
        }
    }

    /// Send an event to the app and return the effects.
    /// # Panics
    /// If the event cannot be deserialized.
    /// In production you should handle the error properly.
    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        match self.core.process_event(data) {
            Ok(effects) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Resolve an effect and return the effects.
    /// # Panics
    /// If the `data` cannot be deserialized into an effect or the `effect_id` is invalid.
    /// In production you should handle the error properly.
    #[must_use]
    pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
        match self.core.handle_response(effect_id, data) {
            Ok(effects) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Get the current `ViewModel`.
    /// # Panics
    /// If the view cannot be serialized.
    /// In production you should handle the error properly.
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        match self.core.view() {
            Ok(view) => view,
            Err(e) => panic!("{e}"),
        }
    }
}
