//! Built-in capability used to notify the Shell that a UI update is necessary.

use std::future::Future;

use serde::{Deserialize, Serialize};

use crate::{
    capability::{CapabilityContext, Operation},
    Capability, Command,
};

/// Use an instance of `Render` to notify the Shell that it should update the user
/// interface. This assumes a declarative UI framework is used in the Shell, which will
/// take the ViewModel provided by [`Core::view`](crate::Core::view) and reconcile the new UI state based
/// on the view model with the previous one.
///
/// For imperative UIs, the Shell will need to understand the difference between the two
/// view models and update the user interface accordingly.
pub struct Render {
    context: CapabilityContext<RenderOperation>,
}

impl Clone for Render {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

/// The single operation `Render` implements.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RenderOperation;

impl Operation for RenderOperation {
    type Output = ();
}

/// Public API of the capability, called by App::update.
impl Render {
    pub fn new(context: CapabilityContext<RenderOperation>) -> Self {
        Self { context }
    }

    /// Call `render` from [`App::update`](crate::App::update) to signal to the Shell that
    /// UI should be re-drawn.
    pub fn render<Event>(&self) -> Command<Event> {
        Command::empty_effect(self.render_async())
    }

    /// Call `render` from [`App::update`](crate::App::update) to signal to the Shell that
    /// UI should be re-drawn.
    pub fn render_async(&self) -> impl Future<Output = ()> {
        let ctx = self.context.clone();
        async move {
            ctx.notify_shell(RenderOperation).await;
        }
    }
}

impl Capability for Render {
    type Operation = RenderOperation;
}
