use super::Command;
use crate::{channels::Sender, Capability};

pub struct Render<Ev> {
    sender: Sender<Command<(), Ev>>,
}

impl<Ev> Render<Ev>
where
    Ev: 'static,
{
    pub fn new(sender: Sender<Command<(), Ev>>) -> Self {
        Self { sender }
    }

    pub fn render(&self) {
        self.sender.send(Command::new_without_callback(()))
    }
} // Public API of the capability, called by App::update.

impl<Ev> Capability for Render<Ev> {}
