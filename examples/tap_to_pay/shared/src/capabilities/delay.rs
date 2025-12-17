use std::future::Future;

use facet::Facet;
use serde::{Deserialize, Serialize};

use crux_core::{Command, Request, capability::Operation, command::RequestBuilder};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
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
