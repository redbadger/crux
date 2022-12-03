use super::Command;
use crate::{channels::Sender, Capability};

pub struct Render<Ev, Ef> {
    sender: Sender<Command<Ef, Ev>>,
    make_effect: fn() -> Ef,
}

impl<Ev, Ef> Render<Ev, Ef>
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

    pub fn render(&self) {
        self.sender
            .send(Command::new_without_callback((self.make_effect)()))
    }
} // Public API of the capability, called by App::update.

impl<Ef, Ev> Capability for Render<Ef, Ev> where Ef: Clone {}
