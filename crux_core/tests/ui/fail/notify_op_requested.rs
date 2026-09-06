//! An operation that declares `Notify` cannot be sent with `request_from_shell`.

use crux_core::{Command, Request, RequestKind, capability::Operation, operation};

struct ANotification;

impl Operation for ANotification {
    type Output = ();
    const KIND: Option<RequestKind> = Some(RequestKind::Notify);
}

impl operation::Notify for ANotification {}

enum Effect {
    Notification(Request<ANotification>),
}

impl From<Request<ANotification>> for Effect {
    fn from(request: Request<ANotification>) -> Self {
        Self::Notification(request)
    }
}

enum Event {
    Done,
}

fn main() {
    let _: Command<Effect, Event> =
        Command::request_from_shell(ANotification).then_send(|()| Event::Done);
}
