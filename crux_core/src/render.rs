//! Built-in capability used to notify the Shell that a UI update is necessary.

use serde::{Deserialize, Serialize};

use crate::{
    capability::{CapabilityContext, Operation},
    Capability,
};

/// Use an instance of `Render` to notify the Shell that it should update the user
/// interface. This assumes a declarative UI framework is used in the Shell, which will
/// take the ViewModel provided by [`Core::view`](crate::Core::view) and reconcile the new UI state based
/// on the view model with the previous one.
///
/// For imperative UIs, the Shell will need to understand the difference between the two
/// view models and update the user interface accordingly.
pub struct Render<Ev> {
    context: CapabilityContext<RenderOperation, Ev>,
}

/// The single operation `Render` implements.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RenderOperation;

impl Operation for RenderOperation {
    type Output = ();
}

/// Public API of the capability, called by App::update.
impl<Ev> Render<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<RenderOperation, Ev>) -> Self {
        Self { context }
    }

    /// Call `render` from [`App::update`](crate::App::update) to signal to the Shell that
    /// UI should be re-drawn.
    pub fn render(&self) {
        let ctx = self.context.clone();
        self.context.spawn(async move {
            ctx.notify_shell(RenderOperation).await;
        });
    }
}

impl<Ev> Capability<Ev> for Render<Ev> {
    type Operation = RenderOperation;

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
