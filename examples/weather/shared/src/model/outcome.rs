use crux_core::Command;

use crate::effects::Effect;

/// Whether the sub-state is still working or has completed.
#[derive(Debug)]
pub(crate) enum Status<S, T> {
    /// The sub-state needs more events.
    Continue(S),
    /// The sub-state is done, here's the result.
    Complete(T),
}

/// The result of `start()`: a new state paired with the command to execute.
pub(crate) struct Started<S, Event> {
    pub state: S,
    pub command: Command<Effect, Event>,
}

impl<S, Event> Started<S, Event> {
    pub fn new(state: S, command: Command<Effect, Event>) -> Self {
        Started { state, command }
    }

    pub fn into_parts(self) -> (S, Command<Effect, Event>) {
        (self.state, self.command)
    }

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

#[cfg(test)]
impl<S, Event> Started<S, Event> {
    pub fn into_state(self) -> S {
        self.state
    }
}

/// The result of `update()`: a status paired with the command to execute.
pub(crate) struct Outcome<S, T, Event> {
    pub status: Status<S, T>,
    pub command: Command<Effect, Event>,
}

impl<S, T, Event> Outcome<S, T, Event> {
    pub fn continuing(state: S, command: Command<Effect, Event>) -> Self {
        Outcome {
            status: Status::Continue(state),
            command,
        }
    }

    pub fn complete(value: T, command: Command<Effect, Event>) -> Self {
        Outcome {
            status: Status::Complete(value),
            command,
        }
    }

    pub fn into_parts(self) -> (Status<S, T>, Command<Effect, Event>) {
        (self.status, self.command)
    }

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

#[cfg(test)]
pub(crate) struct Asserted<V, Event> {
    pub value: V,
    pub command: Command<Effect, Event>,
}

#[cfg(test)]
impl<V, Event> Asserted<V, Event> {
    pub fn into_value(self) -> V {
        self.value
    }

    pub fn into_command(self) -> Command<Effect, Event> {
        self.command
    }

    pub fn into_parts(self) -> (V, Command<Effect, Event>) {
        (self.value, self.command)
    }
}

#[cfg(test)]
impl<S: std::fmt::Debug, T: std::fmt::Debug, Event> Outcome<S, T, Event> {
    pub fn expect_continue(self) -> Asserted<S, Event> {
        assert_let_bind::assert_let!(Status::Continue(state), self.status);
        Asserted { value: state, command: self.command }
    }

    pub fn expect_complete(self) -> Asserted<T, Event> {
        assert_let_bind::assert_let!(Status::Complete(value), self.status);
        Asserted { value, command: self.command }
    }
}
