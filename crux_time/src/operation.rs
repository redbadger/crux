//! One type per time operation.
//!
//! Each type in this module is a single operation the shell can perform, with
//! exactly one output type and a statically declared
//! [`RequestKind`](crux_core::RequestKind). That is the difference from
//! [`TimeRequest`](crate::TimeRequest), where every variant shares one
//! [`TimeResponse`](crate::TimeResponse) and the capability has to check at
//! runtime that the shell answered the question it was asked.
//!
//! Note that [`Clear`] is a notification: it tells the shell to release the
//! resources behind a timer and there is nothing to answer. The old API asked
//! for it with `TimeRequest::Clear` and waited for a `TimeResponse::Cleared`
//! acknowledgement.
//!
//! ```
//! # use crux_core::{Command, macros::effect};
//! use crux_time::{Clock, TimerOutcome, operation};
//!
//! #[effect]
//! enum Effect {
//!     NotifyAfter(operation::NotifyAfter),
//!     Clear(operation::Clear),
//! }
//!
//! # enum Event { Elapsed(TimerOutcome) }
//! let (builder, _handle) = Clock::notify_after(std::time::Duration::from_secs(1));
//! let command: Command<Effect, Event> = builder.then_send(Event::Elapsed);
//! ```

use crux_core::macros::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::protocol::{Duration, Instant, TimerId};

/// Ask for the current wall-clock time.
#[derive(Operation, Facet, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[operation(request, output = Instant)]
pub struct Now;

/// Ask to be notified when `instant` has arrived, answering with `id`.
#[derive(Operation, Facet, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[operation(request, output = TimerId)]
pub struct NotifyAt {
    pub id: TimerId,
    pub instant: Instant,
}

/// Ask to be notified once `duration` has elapsed, answering with `id`.
#[derive(Operation, Facet, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[operation(request, output = TimerId)]
pub struct NotifyAfter {
    pub id: TimerId,
    pub duration: Duration,
}

/// Tell the shell that the timer `id` is no longer wanted, so it can clean up
/// any resources behind it. Nothing is expected in return.
#[derive(Operation, Facet, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[operation(notify)]
pub struct Clear {
    pub id: TimerId,
}
