//! An operation that declares `Request` cannot be sent with `stream_from_shell`.

use crux_core::{Command, Request, RequestKind, capability::Operation, operation};

struct ARequest;

impl Operation for ARequest {
    type Output = usize;
    const KIND: Option<RequestKind> = Some(RequestKind::Request);
}

impl operation::Request for ARequest {}

enum Effect {
    ARequest(Request<ARequest>),
}

impl From<Request<ARequest>> for Effect {
    fn from(request: Request<ARequest>) -> Self {
        Self::ARequest(request)
    }
}

enum Event {
    Done(usize),
}

fn main() {
    let _: Command<Effect, Event> =
        Command::stream_from_shell(ARequest).then_send(Event::Done);
}
