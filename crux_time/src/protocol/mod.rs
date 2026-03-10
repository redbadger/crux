#[cfg(feature = "chrono")]
pub mod chrono;
pub mod duration;
pub mod instant;

use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

pub use duration::Duration;
pub use instant::Instant;

#[derive(Facet, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "native_bridge", derive(uniffi::Enum))]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub enum TimeRequest {
    Now,
    NotifyAt { id: TimerId, instant: Instant },
    NotifyAfter { id: TimerId, duration: Duration },
    Clear { id: TimerId },
}

#[derive(Facet, Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimerId(pub usize);

#[derive(Facet, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "native_bridge", derive(uniffi::Enum))]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub enum TimeResponse {
    Now { instant: Instant },
    InstantArrived { id: TimerId },
    DurationElapsed { id: TimerId },
    Cleared { id: TimerId },
}

impl Operation for TimeRequest {
    type Output = TimeResponse;

    #[cfg(feature = "typegen")]
    fn register_types(
        generator: &mut crux_core::type_generation::serde::TypeGen,
    ) -> crux_core::type_generation::serde::Result {
        generator.register_type::<Instant>()?;
        generator.register_type::<Duration>()?;
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }
}
