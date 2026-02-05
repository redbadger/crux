use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

#[derive(Facet, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "native_bridge", derive(uniffi::Record))]
pub struct PlatformRequest;

// TODO revisit this
#[derive(Facet, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

// Custom type conversion for PlatformResponse (tuple struct -> String)
#[cfg(feature = "native_bridge")]
mod uniffi_compat {
    type PlatformResponse = super::PlatformResponse;
    uniffi::custom_type!(PlatformResponse, String, {
        remote,
        try_lift: |val| Ok(super::PlatformResponse(val)),
        lower: |obj| obj.0.clone(),
    });
}

impl Operation for PlatformRequest {
    type Output = PlatformResponse;
}
