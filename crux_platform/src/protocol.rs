#![allow(deprecated)]

use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

#[deprecated(
    since = "0.10.0",
    note = "The `crux_platform` crate is deprecated. Copy the types into your own project instead. See the README for migration guidance: https://crates.io/crates/crux_platform"
)]
#[derive(Facet, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformRequest;

#[deprecated(
    since = "0.10.0",
    note = "The `crux_platform` crate is deprecated. Copy the types into your own project instead. See the README for migration guidance: https://crates.io/crates/crux_platform"
)]
#[derive(Facet, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

#[allow(deprecated)]
impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}
