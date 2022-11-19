pub use app::*;
use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod app;

// TODO hide this plumbing

uniffi_macros::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Core<CatFacts> = Core::new();
}

#[wasm_bindgen]
pub fn message(data: &[u8]) -> Vec<u8> {
    CORE.message(data)
}

#[wasm_bindgen]
pub fn response(data: &[u8]) -> Vec<u8> {
    CORE.response(data)
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view()
}
