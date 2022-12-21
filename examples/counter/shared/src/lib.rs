pub mod app;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

pub use crux_core::Request;
use crux_core::{capability::CapabilityContext, render::Render, Core};
pub use crux_http as http;
use http::{Http, HttpRequest};

pub use app::*;

// TODO hide this plumbing

uniffi_macros::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Core<Effect, App> = Core::new::<Capabilities>();
}

#[wasm_bindgen]
pub fn message(data: &[u8]) -> Vec<u8> {
    CORE.message(data)
}

#[wasm_bindgen]
pub fn response(uuid: &[u8], data: &[u8]) -> Vec<u8> {
    CORE.response(uuid, data)
}

#[wasm_bindgen]
pub fn view() -> Vec<u8> {
    CORE.view()
}

// TODO macro effect generation:
// crux_macros::generate_effect!(Effect, MyCapabilities);

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Effect {
    Http(HttpRequest),
    Render,
}

impl crux_core::WithContext<App, Effect> for Capabilities {
    fn new_with_context(context: CapabilityContext<Effect, Event>) -> Capabilities {
        Capabilities {
            http: Http::new(context.with_effect(Effect::Http)),
            render: Render::new(context.with_effect(|_| Effect::Render)),
        }
    }
}
