//! TODO mod docs

use crux_core::{channels::Sender, Capability, Command};
use serde::{Deserialize, Serialize};

// TODO revisit this
#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

pub struct Platform<Ev, Ef> {
    sender: Sender<Command<Ef, Ev>>,
    make_effect: fn() -> Ef,
}

impl<Ev, Ef> Platform<Ev, Ef>
where
    Ef: 'static,
    Ev: 'static,
{
    pub fn new(sender: Sender<Command<Ef, Ev>>, make_effect: fn() -> Ef) -> Self {
        Self {
            sender,
            make_effect,
        }
    }

    pub fn get<F>(&self, callback: F)
    where
        F: Fn(PlatformResponse) -> Ev + Send + Sync + 'static,
    {
        self.sender
            .send(Command::new((self.make_effect)(), callback))
    }
}

impl<Ev, Ef> Capability for Platform<Ev, Ef> {}
