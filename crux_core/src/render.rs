use super::Command;
use crate::{channels::Sender, Capability};

pub struct Render<Ef, Ev> {
    sender: Sender<Command<Ef, Ev>>,
    effect: Ef,
}

impl<Ef, Ev> Render<Ef, Ev>
where
    Ef: Clone + 'static,
    Ev: 'static,
{
    pub fn new(sender: Sender<Command<Ef, Ev>>, effect: Ef) -> Self {
        Self { sender, effect }
    }

    pub fn render(&self) {
        self.sender
            .send(Command::new_without_callback(self.effect.clone()))
    }
} // Public API of the capability, called by App::update.

impl<Ef, Ev> super::Capability for Render<Ef, Ev> where Ef: Clone {}
