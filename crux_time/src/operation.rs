//! One type per time operation.
//!
//! Each type in this module is a single operation the shell can perform, with
//! exactly one output type and a statically declared
//! [`OperationKind`](crux_core::OperationKind). That is the difference from
//! [`TimeRequest`](crate::TimeRequest), where every variant shares one
//! [`TimeResponse`](crate::TimeResponse) and the capability has to check at
//! runtime that the shell answered the question it was asked.
//!
//! [`Clear`] is a request like the rest: it asks the shell to release the
//! resources behind a timer, and is answered with the [`TimerId`] it named.
//!
//! [`NotifyEvery`] is a stream operation. It asks for a repeating timer, and
//! the shell answers it with the same [`TimerId`] once each period. The other
//! operations are requests, and the shell answers each of them once.
//!
//! ```
//! # use crux_core::{Command, macros::effect};
//! use crux_time::{TimerOutcome, clock::Time, operation};
//!
//! #[effect]
//! enum Effect {
//!     NotifyAfter(operation::NotifyAfter),
//!     Clear(operation::Clear),
//! }
//!
//! # enum Event { Elapsed(TimerOutcome) }
//! let (builder, _handle) = Time::notify_after(std::time::Duration::from_secs(1));
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

/// Ask to be notified each time `period` elapses, answering with `id` each time.
///
/// This operation is a stream. The shell answers with `id` once per period.
/// The shell continues until a [`Clear`] request for `id` arrives, or until a
/// resolve returns an error. An error tells the shell that the core no longer
/// listens for the ticks.
#[derive(Operation, Facet, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[operation(stream, output = TimerId)]
pub struct NotifyEvery {
    pub id: TimerId,
    pub period: Duration,
}

/// Ask the shell to release the resources behind the timer `id`, because it is
/// no longer wanted, answering with the same `id`.
///
/// The timer's own [`NotifyAt`] or [`NotifyAfter`] request may still be
/// answered after this one — a shell need not race the two. By then the core
/// has stopped listening for that answer, and ignores it.
///
/// For a [`NotifyEvery`] timer, the shell stops the repeating timer when this
/// request arrives. The core stops listening for ticks before it sends the
/// request. If the shell resolves a tick after that, the resolve returns an
/// error, and the shell can ignore that error.
#[derive(Operation, Facet, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[operation(request, output = TimerId)]
pub struct Clear {
    pub id: TimerId,
}
