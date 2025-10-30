//! Built-in capability used to notify the Shell that a UI update is necessary.

use std::future::Future;

use facet::Facet;
use serde::{Deserialize, Serialize};

#[expect(deprecated)]
use crate::{Capability, MaybeSend, MaybeSync, capability::CapabilityContext};
use crate::{Command, Request, capability::Operation, command::NotificationBuilder};

/// Use an instance of `Render` to notify the Shell that it should update the user
/// interface. This assumes a declarative UI framework is used in the Shell, which will
/// take the `ViewModel` provided by [`Core::view`](crate::Core::view) and reconcile the new UI state based
/// on the view model with the previous one.
///
/// For imperative UIs, the Shell will need to understand the difference between the two
/// view models and update the user interface accordingly.
#[deprecated(
    since = "0.16.0",
    note = "The Render capability has been deprecated. Use render::render() with the Command API instead."
)]
pub struct Render<Ev> {
    #[expect(deprecated)]
    context: CapabilityContext<RenderOperation, Ev>,
}

#[expect(deprecated)]
impl<Ev> Clone for Render<Ev> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

/// The single operation `Render` implements.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RenderOperation;

impl Operation for RenderOperation {
    type Output = ();
}

/// Public API of the capability, called by `App::update`.
#[expect(deprecated)]
impl<Ev> Render<Ev>
where
    Ev: 'static,
{
    #[must_use]
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

#[expect(deprecated)]
impl<Ev> Capability<Ev> for Render<Ev> {
    type Operation = RenderOperation;
    type MappedSelf<MappedEv> = Render<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + MaybeSend + MaybeSync + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Render::new(self.context.map_event(f))
    }
}

/// Signal to the shell that the UI should be redrawn.
/// Returns a [`NotificationBuilder`].
///
/// ### Examples:
/// To use in a sync context:
/// ```
///# use crux_core::{Command, render::{render_builder, Render, RenderOperation}};
///# #[crux_core::macros::effect]pub enum Effect {Render(RenderOperation)}
///# enum Event {None}
/// let command: Command<Effect, Event> =
///     render_builder().into(); // or use `render_command()`
/// ```
/// To use in an async context:
/// ```
///# use crux_core::{Command, render::{render_builder, Render, RenderOperation}};
///# #[crux_core::macros::effect]pub enum Effect {Render(RenderOperation)}
///# enum Event {None}
///# let command: Command<Effect, Event> = Command::new(|ctx| async move {
/// render_builder().into_future(ctx).await;
///# });
/// ```
#[must_use]
pub fn render_builder<Effect, Event>()
-> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
where
    Effect: From<Request<RenderOperation>> + MaybeSend + 'static,
    Event: MaybeSend + 'static,
{
    Command::notify_shell(RenderOperation)
}

/// Signal to the shell that the UI should be redrawn.
/// Returns a [`Command`].
pub fn render<Effect, Event>() -> Command<Effect, Event>
where
    Effect: From<Request<RenderOperation>> + MaybeSend + 'static,
    Event: MaybeSend + 'static,
{
    render_builder().into()
}
