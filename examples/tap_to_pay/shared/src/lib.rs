mod app;
mod capabilities;

pub use app::*;
pub use capabilities::delay::DelayOperation;
pub use crux_core::Request;
use crux_core::{bridge::Bridge, Core};
use lazy_static::lazy_static;

uniffi::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Bridge<Effect, App> = Bridge::new(Core::new::<Capabilities>());
}

pub fn process_event(data: &[u8]) -> Vec<u8> {
    CORE.process_event(data)
}

pub fn handle_response(uuid: &[u8], data: &[u8]) -> Vec<u8> {
    CORE.handle_response(uuid, data)
}

pub fn view() -> Vec<u8> {
    CORE.view()
}
