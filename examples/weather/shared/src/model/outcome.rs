use crux_core::Command;

use crate::effects::Effect;

/// The result of `update()` when a sub-state can either keep running or transition.
pub(crate) enum Outcome<S, T, Event> {
    /// The sub-state stays. The parent puts it back.
    Continue(S, Command<Effect, Event>),
    /// The sub-state is consumed. The parent uses the transition value.
    Complete(T, Command<Effect, Event>),
}

impl<S, T, Event> Outcome<S, T, Event> {
    pub fn map_event<NewEvent>(
        self,
        f: impl Fn(Event) -> NewEvent + Send + Sync + 'static,
    ) -> Outcome<S, T, NewEvent>
    where
        Event: Send + Unpin + 'static,
        NewEvent: Send + Unpin + 'static,
    {
        match self {
            Outcome::Continue(state, command) => Outcome::Continue(state, command.map_event(f)),
            Outcome::Complete(value, command) => Outcome::Complete(value, command.map_event(f)),
        }
    }
}
