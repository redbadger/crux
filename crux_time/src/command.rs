#[allow(deprecated)]
#[deprecated(since = "0.15.0", note = "Import directly from crate root")]
pub use crate::{CompletedTimerHandle, TimerHandle, TimerOutcome};
// spike(no_std): the deprecated root `Time` needs std.
#[cfg(feature = "std")]
#[allow(deprecated)]
#[deprecated(since = "0.15.0", note = "Import directly from crate root")]
pub use crate::Time;
