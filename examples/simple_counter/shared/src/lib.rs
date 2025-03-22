pub mod app;
mod capabilities;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys;

use std::sync::{Arc, Mutex, OnceLock};

pub use app::*;
use crux_core::bridge::Bridge;
pub use crux_core::Middleware;
pub use crux_core::{
    bridge::{Nacre, NacreBridge, Shell},
    Request,
};

uniffi::include_scaffolding!("shared");

static CORE: OnceLock<Bridge<NacreBridge<Counter>>> = OnceLock::new();

pub struct JsShell {
    handle_effects: Arc<Mutex<js_sys::Function>>,
}

// SAFETY: All callback instances are wrapped into Arc<Mutex> so this is safe to mark
unsafe impl Send for JsShell {}
// SAFETY: All callback instances are wrapped into Arc<Mutex> so this is safe to mark
unsafe impl Sync for JsShell {}

impl JsShell {
    pub fn new(handle_effects: js_sys::Function) -> Self {
        Self {
            handle_effects: Arc::new(handle_effects.into()),
        }
    }
}

impl Shell for JsShell {
    fn handle_effects(&self, effects: Vec<u8>) {
        self.handle_effects
            .lock()
            .unwrap()
            .call1(&JsValue::null(), &JsValue::from(effects))
            .unwrap();
    }
}

#[wasm_bindgen]
pub fn init(handle_effects: js_sys::Function) {
    new(Arc::new(JsShell::new(handle_effects)));
}

pub fn new(shell: Arc<dyn Shell>) {
    let (sender, receiver) = async_std::channel::bounded(1);
    let nacre = NacreBridge::new(sender, shell);
    let _ = CORE.set(Bridge::from_nacre(nacre, receiver));
}

#[wasm_bindgen]
pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE.get().unwrap().process_event(data).unwrap()
}

#[wasm_bindgen]
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> {
    CORE.get().unwrap().handle_response(id, data).unwrap()
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.get().unwrap().view().unwrap()
}
