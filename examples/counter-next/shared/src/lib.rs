mod app;
mod capabilities;

pub use crux_core::Core;
use crux_core::bridge::{Bridge, EffectId};
use crux_core::middleware::{self, BincodeFfiFormat, Layer as _};

pub use crux_http as http;

use std::sync::{Arc, LazyLock};

pub use app::*;
pub use capabilities::sse;

static CORE: LazyLock<Bridge<App>> = LazyLock::new(|| Bridge::new(Core::new()));

#[cfg(not(target_family = "wasm"))]
const _: () = assert!(
    uniffi::check_compatible_version("0.29.3"),
    "please use uniffi v0.29.3"
);

#[cfg(not(target_family = "wasm"))]
uniffi::setup_scaffolding!();

/// Ask the core to process an event
/// # Panics
/// If the core fails to process the event
#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    match CORE.process_event(data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

/// Ask the core to handle a response
/// # Panics
/// If the core fails to handle the response
#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    match CORE.handle_response(id, data) {
        Ok(effects) => effects,
        Err(e) => panic!("{e}"),
    }
}

/// Ask the core to render the view
/// # Panics
/// If the view cannot be serialized
#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub fn view() -> Vec<u8> {
    match CORE.view() {
        Ok(view) => view,
        Err(e) => panic!("{e}"),
    }
}

// ---- new FFI ---

/// For the Shell to provide
/// TODO: Move to Crux
#[cfg_attr(not(target_family = "wasm"), uniffi::export(with_foreign))]
pub trait CruxShell: Send + Sync {
    /// Called when any effects resulting from an asynchronous process
    /// need processing by the shell.
    ///
    /// The bytes are a serialized vector of requests
    fn process_effects(&self, bytes: Vec<u8>);
}

/// The main interface used by the shell
#[cfg_attr(not(target_family = "wasm"), derive(uniffi::Object))]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct CoreFFI {
    core: middleware::Bridge<Core<App>, BincodeFfiFormat>,
}

#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl CoreFFI {
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        match self.core.update(data) {
            Ok(effects) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    pub fn resolve(&self, effect_id: u32, data: &[u8]) -> Vec<u8> {
        match self.core.resolve(EffectId(effect_id), data) {
            Ok(view) => view,
            Err(e) => panic!("{e}"),
        }
    }

    pub fn view(&self) -> Vec<u8> {
        match self.core.view() {
            Ok(view) => view,
            Err(e) => panic!("{e}"),
        }
    }
}

// UniFFI constructor

#[cfg(not(target_family = "wasm"))]
#[cfg_attr(not(target_family = "wasm"), uniffi::export)]
impl CoreFFI {
    #[uniffi::constructor]
    pub fn new(shell: Arc<dyn CruxShell>) -> Self {
        let core =
            Core::<App>::new().bridge::<BincodeFfiFormat>(move |effect_bytes| match effect_bytes {
                Ok(effect) => shell.process_effects(effect),
                Err(e) => panic!("{e}"),
            });

        Self { core }
    }
}

// WebAssembly constructor

#[cfg(target_family = "wasm")]
struct JsCallback(js_sys::Function);

#[cfg(target_family = "wasm")]
unsafe impl Send for JsCallback {}

#[cfg(target_family = "wasm")]
unsafe impl Sync for JsCallback {}

#[cfg(target_family = "wasm")]
impl std::ops::Deref for JsCallback {
    type Target = js_sys::Function;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(target_family = "wasm")]
#[cfg_attr(target_family = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
impl CoreFFI {
    #[wasm_bindgen::prelude::wasm_bindgen(constructor)]
    pub fn new(callback: js_sys::Function) -> Self {
        use js_sys::wasm_bindgen::JsValue;

        let callback = JsCallback(callback);
        let core =
            Core::<App>::new().bridge::<BincodeFfiFormat>(move |effect_bytes| match effect_bytes {
                Ok(bytes) => {
                    callback
                        .call1(&JsValue::NULL, &JsValue::from(bytes))
                        .expect("Could not call JS callback");
                }
                Err(e) => {
                    panic!("{e}");
                }
            });

        Self { core }
    }
}
