#[cfg(feature = "chrono")]
pub mod chrono;
pub mod duration;
pub mod instant;

use crux_core::capability::Operation;
use serde::{Deserialize, Serialize};

use duration::Duration;
use instant::Instant;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeRequest {
    Now,
    NotifyAt { id: TimerId, instant: Instant },
    NotifyAfter { id: TimerId, duration: Duration },
    Clear { id: TimerId },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimerId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimeResponse {
    Now { instant: Instant },
    InstantArrived { id: TimerId },
    DurationElapsed { id: TimerId },
    Cleared { id: TimerId },
}

impl Operation for TimeRequest {
    type Output = TimeResponse;
}
