use lazy_static::lazy_static;
use wasm_bindgen::prelude::wasm_bindgen;

pub mod app;

pub use app::*;

// TODO hide this plumbing

uniffi_macros::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: AppCore<CatFacts> = Default::default();
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
