use std::future::Future;

use crux_core::{command::RequestBuilder, Command, Request};
use serde::{Deserialize, Serialize};

// ANCHOR: operation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DelayOperation {
    millis: usize,
}
// ANCHOR_END: operation

// ANCHOR: operation_impl
impl crux_core::capability::Operation for DelayOperation {
    type Output = ();
}
// ANCHOR_END: operation_impl

// ANCHOR: functions
/// Request a delay for the specified number of milliseconds.
#[must_use]
pub fn milliseconds<Effect, Event>(
    millis: usize,
) -> RequestBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: Send + From<Request<DelayOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(DelayOperation { millis })
}
// ANCHOR_END: functions

// ANCHOR: tests
#[cfg(test)]
mod tests {
    use crux_core::macros::effect;

    use super::*;

    #[effect]
    pub enum Effect {
        Delay(DelayOperation),
    }

    enum Event {
        TimeUp(()),
    }

    #[test]
    fn test_delay() {
        let delay = 100;

        let mut cmd = milliseconds(delay).then_send(Event::TimeUp);

        let effect = cmd.expect_one_effect();
        let Effect::Delay(mut request) = effect;

        assert_eq!(request.operation, DelayOperation { millis: delay });

        request.resolve(()).unwrap();

        let event = cmd.events().next().unwrap();
        assert!(matches!(event, Event::TimeUp(())));

        assert!(cmd.is_done());
    }
}
// ANCHOR_END: tests
