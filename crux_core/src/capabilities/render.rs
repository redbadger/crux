//! Built-in capability used to notify the Shell that a UI update is necessary.

use std::future::Future;

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    Command, Request, bridge::UnitOutput, capability::Operation, command::NotificationBuilder,
};

/// The single operation `Render` implements.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "native_bridge", derive(uniffi::Record))]
pub struct RenderOperation;

impl Operation for RenderOperation {
    type Output = UnitOutput;
}

/// Signal to the shell that the UI should be redrawn.
/// Returns a [`NotificationBuilder`].
///
/// ### Examples:
/// To use in a sync context:
/// ```
///# use crux_core::{Command, render::{render_builder, RenderOperation}};
///# #[crux_core::macros::effect]pub enum Effect {Render(RenderOperation)}
///# enum Event {None}
/// let command: Command<Effect, Event> =
///     render_builder().into(); // or use `render_command()`
/// ```
/// To use in an async context:
/// ```
///# use crux_core::{Command, render::{render_builder, RenderOperation}};
///# #[crux_core::macros::effect]pub enum Effect {Render(RenderOperation)}
///# enum Event {None}
///# let command: Command<Effect, Event> = Command::new(|ctx| async move {
/// render_builder().into_future(ctx).await;
///# });
/// ```
#[must_use]
pub fn render_builder<Effect, Event>()
-> NotificationBuilder<Effect, Event, impl Future<Output = UnitOutput>>
where
    Effect: From<Request<RenderOperation>> + Send + 'static,
    Event: Send + 'static,
{
    Command::notify_shell(RenderOperation)
}

/// Signal to the shell that the UI should be redrawn.
/// Returns a [`Command`].
pub fn render<Effect, Event>() -> Command<Effect, Event>
where
    Effect: From<Request<RenderOperation>> + Send + 'static,
    Event: Send + 'static,
{
    render_builder().into()
}
