use std::future::Future;

use crux_core::{capability::Operation, command::RequestBuilder, Command, Request};
use serde::{Deserialize, Serialize};

// ANCHOR: operation
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    GetRandom(usize, usize),
    Delay(usize),
}
// ANCHOR_END: operation

// ANCHOR: output
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOutput {
    Random(usize),
    TimeUp,
}
// ANCHOR_END: output

// ANCHOR: operation_impl
impl Operation for DelayOperation {
    type Output = DelayOutput;
}
// ANCHOR_END: operation_impl

// ANCHOR: functions
/// Request a delay for the specified number of milliseconds.
#[must_use]
pub fn milliseconds<Effect, Event>(
    millis: usize,
) -> RequestBuilder<Effect, Event, impl Future<Output = DelayOutput>>
where
    Effect: Send + From<Request<DelayOperation>> + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(DelayOperation::Delay(millis))
}

/// Request a delay for a random duration between `min` and `max` milliseconds.
///
/// # Panics
///
/// - will panic if `min` is greater than `max`.
/// - will panic if the shell responds with the incorrect [`DelayOutput`] variant.
#[must_use]
pub fn random<Effect, Event>(
    min: usize,
    max: usize,
) -> RequestBuilder<Effect, Event, impl Future<Output = DelayOutput>>
where
    Effect: Send + From<Request<DelayOperation>> + 'static,
    Event: Send + 'static,
{
    assert!(min <= max, "min must be less than or equal to max");

    Command::request_from_shell(DelayOperation::GetRandom(min, max)).then_request(|response| {
        let DelayOutput::Random(millis) = response else {
            panic!("Expected a random number")
        };

        Command::request_from_shell(DelayOperation::Delay(millis))
    })
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
        Delay(DelayOutput),
    }

    #[test]
    fn test_delay() {
        let delay = 100;

        let mut cmd = milliseconds(delay).then_send(Event::Delay);

        let effect = cmd.expect_one_effect();
        let Effect::Delay(mut request) = effect;

        assert_eq!(request.operation, DelayOperation::Delay(delay));

        request.resolve(DelayOutput::TimeUp).unwrap();

        let event = cmd.events().next().unwrap();
        let Event::Delay(output) = event;
        assert_eq!(output, DelayOutput::TimeUp);

        assert!(cmd.is_done());
    }

    #[test]
    fn test_random() {
        let min = 100;
        let max = 200;

        let mut cmd = random(min, max).then_send(Event::Delay);

        let effect = cmd.expect_one_effect();
        let Effect::Delay(mut request) = effect;

        assert_eq!(request.operation, DelayOperation::GetRandom(min, max));
        request.resolve(DelayOutput::Random(150)).unwrap();

        let effect = cmd.expect_one_effect();
        let Effect::Delay(mut request) = effect;

        assert_eq!(request.operation, DelayOperation::Delay(150));

        request.resolve(DelayOutput::TimeUp).unwrap();

        let event = cmd.events().next().unwrap();
        let Event::Delay(output) = event;
        assert_eq!(output, DelayOutput::TimeUp);

        assert!(cmd.is_done());
    }
}
// ANCHOR_END: tests
