//! TODO mod docs

use crux_core::{capability::CapabilityContext, channels::Sender, Capability, Command};
use serde::{Deserialize, Serialize};

// TODO revisit this
#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResponse(pub String);

pub struct Time<Ev> {
    context: CapabilityContext<(), Ev>,
}

impl<Ev> Time<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<(), Ev>) -> Self {
        Self { context }
    }

    pub fn get<F>(&self, callback: F)
    where
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        self.context.run_command(Command::new((), callback))
    }
}

impl<Ef> Capability<Ef> for Time<Ef> {
    type MappedSelf<MappedEv> = Time<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ef + Send + Sync + Copy + 'static,
        Ef: 'static,
        NewEvent: 'static,
    {
        Time::new(self.context.map_event(f))
    }
}
