use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformRequest;

// TODO revisit this
#[derive(Facet, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}
