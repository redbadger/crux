pub mod app;
pub mod capabilities;

use lazy_static::lazy_static;

use crux_core::Core;
pub use crux_core::Request;

pub use app::*;

uniffi::include_scaffolding!("shared");

lazy_static! {
    static ref CORE: Core<Effect, NoteEditor> = Core::new::<Capabilities>();
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
