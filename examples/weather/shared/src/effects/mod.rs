//! Effects — the shell-crossing boundary of the app.
//!
//! [`Effect`] enumerates every side-effect the core can request from the
//! shell. Three of the variants wrap Crux's built-in capabilities ([`Render`],
//! [`KeyValue`], [`Http`], [`Time`]); two wrap custom capabilities defined
//! here ([`location`], [`secret`]). The [`http`] submodule contains the
//! typed clients the model uses to talk to OpenWeatherMap.
//!
//! [`Render`]: crux_core::render::RenderOperation
//! [`KeyValue`]: crux_kv::KeyValueOperation
//! [`Http`]: crux_http::protocol::HttpRequest
//! [`Time`]: crux_time::TimeRequest

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
/// Every side-effect the core can ask the shell to perform.
///
/// Each variant is a request the shell fulfils and then resolves, producing
/// an event that the model handles. The `#[effect(facet_typegen)]` macro
/// generates the FFI glue that drives that exchange.
#[effect(facet_typegen)]
pub enum Effect {
    /// Ask the shell to re-read the [`ViewModel`](crate::ViewModel) and
    /// repaint.
    Render(RenderOperation),
    /// Read, write, or delete a value in the shell's key-value store.
    /// Used to persist the favourites list.
    KeyValue(KeyValueOperation),
    /// Perform an HTTP request — weather and geocoding API calls.
    Http(HttpRequest),
    /// Check location permissions or fetch the device's coordinates.
    Location(LocationOperation),
    /// Store, fetch, or delete a secret (the OpenWeatherMap API key).
    Secret(SecretRequest),
    /// Schedule a timer — used to debounce the search input on the
    /// add-favourite screen.
    Time(TimeRequest),
}
// ANCHOR_END: effect
