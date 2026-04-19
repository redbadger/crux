//! State-machine plumbing shared by every nested model in the weather app.
//!
//! Each stage — `InitializingModel`, `OnboardModel`, `ActiveModel`, and the
//! screens and workflows nested inside `Active` — is a small state machine.
//! These types are the protocol that lets them compose:
//!
//! - [`Started`] — the result of a `start()` call: an initial state bundled
//!   with the commands needed to drive it.
//! - [`Outcome`] — the result of an `update()` call: a [`Status`] paired
//!   with a `Command` describing the effects produced by the update.
//! - [`Status`] — whether the state machine is still running (`Continue`)
//!   or has exited with a transition value (`Complete`).
//!
//! A parent state machine delegates events to its child by calling the
//! child's `update`, receives an `Outcome`, matches on `Status` to decide
//! whether to keep the child (`Continue`) or replace it based on the
//! transition (`Complete`), and runs the returned command.

use crux_core::Command;

use crate::effects::Effect;

// ANCHOR: status
/// Whether a state-machine step kept running or exited with a transition.
///
/// Returned inside an [`Outcome`], typically constructed indirectly via
/// [`Outcome::continuing`] or [`Outcome::complete`].
#[derive(Debug)]
pub(crate) enum Status<S, T> {
    /// The state machine is still running; this is the updated state to
    /// assign back into the parent.
    Continue(S),
    /// The state machine has exited; this is the transition value carrying
    /// whatever the parent needs to move to the next phase.
    Complete(T),
}
// ANCHOR_END: status

// ANCHOR: started
/// The result of a state machine's `start()` method.
///
/// A `start()` both constructs the initial state and returns the commands
/// that must run alongside it — HTTP fetches, permission checks, a render.
/// `Started` bundles those so the caller can destructure them in one step
/// with [`Started::into_parts`].
///
/// Use [`Started::map_event`] to lift the inner command's event type into
/// a wider parent event before returning it from the parent's own logic.
pub(crate) struct Started<S, Event> {
    pub state: S,
    pub command: Command<Effect, Event>,
}

impl<S, Event> Started<S, Event> {
    /// Creates a new `Started` from an initial state and its accompanying
    /// command.
    pub fn new(state: S, command: Command<Effect, Event>) -> Self {
        Started { state, command }
    }

    /// Destructures into the initial state and the command to run.
    pub fn into_parts(self) -> (S, Command<Effect, Event>) {
        (self.state, self.command)
    }

    /// Lifts the event type of the inner command.
    ///
    /// Typically used by a parent to wrap the child's event variant, for
    /// example `child::start().map_event(ParentEvent::Child)`.
    pub fn map_event<NewEvent>(
        self,
        f: impl Fn(Event) -> NewEvent + Send + Sync + 'static,
    ) -> Started<S, NewEvent>
    where
        Event: Send + Unpin + 'static,
        NewEvent: Send + Unpin + 'static,
    {
        Started {
            state: self.state,
            command: self.command.map_event(f),
        }
    }
}
// ANCHOR_END: started

#[cfg(test)]
impl<S, Event> Started<S, Event> {
    /// Returns only the state, dropping the command (test helper).
    pub fn into_value(self) -> S {
        self.state
    }
}

// ANCHOR: outcome
/// The result of a state machine's `update()` method.
///
/// Pairs a [`Status`] — continue with an updated state, or complete with
/// a transition value — with a `Command` describing the effects produced
/// by the update. The parent destructures it, reacts to the status, and
/// runs the command.
///
/// Construct with [`Outcome::continuing`] or [`Outcome::complete`]. Use
/// [`Outcome::map_event`] to lift the inner command's event type before
/// returning it from the parent's own update.
pub(crate) struct Outcome<S, T, Event> {
    pub status: Status<S, T>,
    pub command: Command<Effect, Event>,
}

impl<S, T, Event> Outcome<S, T, Event> {
    /// Constructs an outcome that keeps the state machine running with the
    /// given updated state and command.
    pub fn continuing(state: S, command: Command<Effect, Event>) -> Self {
        Outcome {
            status: Status::Continue(state),
            command,
        }
    }

    /// Constructs an outcome that exits the state machine with the given
    /// transition value and command.
    pub fn complete(value: T, command: Command<Effect, Event>) -> Self {
        Outcome {
            status: Status::Complete(value),
            command,
        }
    }

    /// Destructures into the status and the command to run.
    pub fn into_parts(self) -> (Status<S, T>, Command<Effect, Event>) {
        (self.status, self.command)
    }

    /// Lifts the event type of the inner command.
    ///
    /// Typically used by a parent to wrap the child's event variant before
    /// returning the command from its own update.
    pub fn map_event<NewEvent>(
        self,
        f: impl Fn(Event) -> NewEvent + Send + Sync + 'static,
    ) -> Outcome<S, T, NewEvent>
    where
        Event: Send + Unpin + 'static,
        NewEvent: Send + Unpin + 'static,
    {
        Outcome {
            status: self.status,
            command: self.command.map_event(f),
        }
    }
}
// ANCHOR_END: outcome

/// The destructured result of an assertion on an [`Outcome`] (test helper).
///
/// Produced by [`Outcome::expect_continue`] or [`Outcome::expect_complete`]
/// after asserting the status variant. Carries the inner value alongside
/// the command so tests can inspect either.
#[cfg(test)]
pub(crate) struct Asserted<V, Event> {
    pub value: V,
    pub command: Command<Effect, Event>,
}

#[cfg(test)]
impl<V, Event> Asserted<V, Event> {
    /// Returns only the value, dropping the command.
    pub fn into_value(self) -> V {
        self.value
    }

    /// Returns only the command, dropping the value.
    pub fn into_command(self) -> Command<Effect, Event> {
        self.command
    }

    /// Destructures into the value and the command.
    pub fn into_parts(self) -> (V, Command<Effect, Event>) {
        (self.value, self.command)
    }
}

#[cfg(test)]
impl<S: std::fmt::Debug, T: std::fmt::Debug, Event> Outcome<S, T, Event> {
    /// Asserts the outcome is [`Status::Continue`] and returns the state
    /// and command as an [`Asserted`]. Panics otherwise.
    pub fn expect_continue(self) -> Asserted<S, Event> {
        assert_let_bind::assert_let!(Status::Continue(state), self.status);
        Asserted {
            value: state,
            command: self.command,
        }
    }

    /// Asserts the outcome is [`Status::Complete`] and returns the
    /// transition and command as an [`Asserted`]. Panics otherwise.
    pub fn expect_complete(self) -> Asserted<T, Event> {
        assert_let_bind::assert_let!(Status::Complete(value), self.status);
        Asserted {
            value,
            command: self.command,
        }
    }
}
