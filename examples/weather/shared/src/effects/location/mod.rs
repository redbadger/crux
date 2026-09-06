//! A custom capability for accessing the device's location.
//!
//! Two operations, each with its own output: [`IsLocationEnabled`] is answered
//! with a `bool`, [`GetLocation`] with the coordinates the shell managed to
//! obtain — or `None` if it couldn't. There is no shared response enum, so
//! neither the shell nor the core can answer one question with the other's
//! answer. The developer-facing command builders live in the [`command`]
//! submodule.

pub mod command;

use crux_core::macros::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

/// Geographic coordinates as returned by the shell.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

/// Ask whether location services are currently enabled and authorised.
#[derive(Operation, Facet, Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[operation(request, output = bool)]
pub struct IsLocationEnabled;

/// Ask for the device's current coordinates.
#[derive(Operation, Facet, Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[operation(request, output = Option<Location>)]
pub struct GetLocation;
