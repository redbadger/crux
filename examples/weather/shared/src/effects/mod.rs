//! Effects — the shell-crossing boundary of the app.
//!
//! [`Effect`] enumerates every side-effect the core can request from the
//! shell, one operation type per variant. Some come from Crux's built-in
//! capabilities ([`Render`], [`crux_kv`], [`Http`], [`crux_time`]); the rest
//! come from the custom capabilities defined here ([`location`], [`secret`]).
//! The [`http`] submodule contains the typed clients the model uses to talk
//! to OpenWeatherMap.
//!
//! [`Render`]: crux_core::render::RenderOperation
//! [`Http`]: crux_http::protocol::HttpRequest

pub mod http;
pub mod location;
pub mod secret;

use crux_core::{macros::effect, render::RenderOperation};
use crux_http::protocol::HttpRequest;
use crux_kv::operation as kv;
use crux_time::operation as time;

use crate::effects::location::{GetLocation, IsLocationEnabled};
use crate::effects::secret::{Delete, Fetch, Store};

// ANCHOR: effect
/// Every side-effect the core can ask the shell to perform.
///
/// Each variant carries one operation type, which declares both the single
/// output it is answered with and how many times the shell resolves it — a
/// notification never, a request exactly once. The
/// `#[effect(facet_typegen)]` macro generates the FFI glue, and type
/// generation turns this enum into the shell's `EffectHandler`. The app lists
/// only the operations it uses, so the shell is never asked to implement a
/// key-value `ListKeys` it will never see.
#[effect(facet_typegen)]
pub enum Effect {
    /// Ask the shell to re-read the [`ViewModel`](crate::ViewModel) and
    /// repaint.
    Render(RenderOperation),
    /// Perform an HTTP request — weather and geocoding API calls.
    Http(HttpRequest),
    /// Read the favourites list from the shell's key-value store.
    KvGet(kv::Get),
    /// Write the favourites list to the shell's key-value store.
    KvSet(kv::Set),
    /// Schedule a timer — used to debounce the search input on the
    /// add-favourite screen.
    TimeNotifyAfter(time::NotifyAfter),
    /// Release a timer the core no longer cares about.
    TimeClear(time::Clear),
    /// Ask whether location services are enabled.
    IsLocationEnabled(IsLocationEnabled),
    /// Ask for the device's coordinates.
    GetLocation(GetLocation),
    /// Fetch a secret (the OpenWeatherMap API key).
    FetchSecret(Fetch),
    /// Store a secret.
    StoreSecret(Store),
    /// Delete a secret.
    DeleteSecret(Delete),
}
// ANCHOR_END: effect
