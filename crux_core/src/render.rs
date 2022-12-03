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

impl<Ev> Capability<Ev> for Render<Ev> {
    type MappedSelf<Ev2> = Render<Ev2>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static,
    {
        Render::new(self.sender.map_event(f))
    }
}
