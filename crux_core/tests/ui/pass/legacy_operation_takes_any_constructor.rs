//! An operation that declares no kind keeps today's behaviour: all three
//! constructors accept it.

use crux_core::{Command, Request, capability::Operation};

struct AnOperation;

impl Operation for AnOperation {
    type Output = ();
}

enum Effect {
    AnEffect(Request<AnOperation>),
}

impl From<Request<AnOperation>> for Effect {
    fn from(request: Request<AnOperation>) -> Self {
        Self::AnEffect(request)
    }
}

enum Event {
    Done,
}

fn main() {
    let _: Command<Effect, Event> = Command::notify_shell(AnOperation).into();
    let _: Command<Effect, Event> =
        Command::request_from_shell(AnOperation).then_send(|()| Event::Done);
    let _: Command<Effect, Event> =
        Command::stream_from_shell(AnOperation).then_send(|()| Event::Done);
}
