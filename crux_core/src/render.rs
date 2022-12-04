use super::Command;
use crate::{capability::CapabilityContext, Capability};

pub struct Render<Ev> {
    context: CapabilityContext<(), Ev>,
}

impl<Ev> Render<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<(), Ev>) -> Self {
        Self { context }
    }

    pub fn render(&self) {
        self.context.run_command(Command::new_without_callback(()))
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
        Render::new(self.context.map_event(f))
    }
}
