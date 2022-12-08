use serde::Serialize;

use crate::{
    capability::{CapabilityContext, Operation},
    Capability,
};

pub struct Render<Ev> {
    context: CapabilityContext<RenderOperation, Ev>,
}

#[derive(Serialize)]
pub struct RenderOperation;

impl Operation for RenderOperation {
    type Output = ();
}

impl<Ev> Render<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<RenderOperation, Ev>) -> Self {
        Self { context }
    }

    pub fn render(&self) {
        self.context.notify_shell(RenderOperation {})
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
