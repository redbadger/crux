//! `#[derive(Operation)]` with `#[operation(notify)]` declares
//! `RequestKind::Notify`, so `request_from_shell` is a compile error.

use crux_core::{Command, Request, macros::Operation};
use serde::{Deserialize, Serialize};

#[derive(Operation, Debug, Clone, Serialize, Deserialize)]
#[operation(notify)]
struct Publish(Vec<u8>);

enum Effect {
    Publish(Request<Publish>),
}

impl From<Request<Publish>> for Effect {
    fn from(request: Request<Publish>) -> Self {
        Self::Publish(request)
    }
}

enum Event {
    Done,
}

fn main() {
    let _: Command<Effect, Event> =
        Command::request_from_shell(Publish(vec![])).then_send(|()| Event::Done);
}
