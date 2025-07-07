use std::future::Future;

use serde::{Deserialize, Serialize};

use crux_core::{capability::Operation, command::RequestBuilder, Command, Request};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum DelayOperation {
    Start { millis: usize },
}

impl Operation for DelayOperation {
    type Output = ();
}

pub fn delay<Effect, Event>(
    millis: usize,
) -> RequestBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: From<Request<DelayOperation>> + Send + 'static,
    Event: Send + 'static,
{
    Command::request_from_shell(DelayOperation::Start { millis })
}
