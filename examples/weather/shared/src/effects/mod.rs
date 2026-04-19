pub mod http;
pub mod location;
pub mod secret;

use crux_core::{macros::effect, render::RenderOperation};
use crux_http::protocol::HttpRequest;
use crux_kv::KeyValueOperation;
use crux_time::TimeRequest;

use crate::effects::location::LocationOperation;
use crate::effects::secret::SecretRequest;

// ANCHOR: effect
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    KeyValue(KeyValueOperation),
    Http(HttpRequest),
    Location(LocationOperation),
    Secret(SecretRequest),
    Time(TimeRequest),
}
// ANCHOR_END: effect
