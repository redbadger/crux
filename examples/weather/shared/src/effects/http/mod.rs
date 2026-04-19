//! Typed HTTP clients for the OpenWeatherMap API.
//!
//! - [`location`] — geocoding (city name → coordinates).
//! - [`weather`] — current weather at a coordinate.
//!
//! Each submodule exposes a `fetch` function that builds a Crux HTTP command
//! and narrows the response into a domain error type. The response structs
//! mirror the OpenWeatherMap JSON shapes; see the
//! [OpenWeatherMap docs](https://openweathermap.org/api) for field
//! semantics.

pub mod location;
pub mod weather;
