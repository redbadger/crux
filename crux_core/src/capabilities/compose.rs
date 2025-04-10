//! A capability which can spawn tasks which orchestrate across other capabilities. This
//! is useful for orchestrating a number of different effects into a single transaction.

use crate::capability::{CapabilityContext, Never};
use crate::Capability;
use futures::Future;

/// Compose capability can be used to orchestrate effects into a single transaction.
///
/// Example include:
/// * Running a number of HTTP requests in parallel and waiting for all to finish
/// * Chaining effects together, where the output of one is the input of the next and the intermediate
///   results are not useful to the app
/// * Implementing request timeouts by selecting across a HTTP effect and a time effect
/// * Any arbitrary graph of effects which depend on each other (or not).
///
/// The compose capability doesn't have any operations it emits to the shell, and type generation fails
/// on its operation type ([`Never`]). This is difficult for crux to detect
/// at the moment. To avoid this problem until a better fix is found, use `#[effect(skip)]` to skip the
/// generation of an effect variant for the compose capability. For example
///
/// ```rust
/// # use crux_core::macros::Effect;
/// # use crux_core::{compose::Compose, render::Render};
/// # enum Event { Nothing }
/// #[derive(Effect)]
/// pub struct Capabilities {
///     pub render: Render<Event>,
///     #[effect(skip)]
///     pub compose: Compose<Event>,
/// }
/// ```
///
/// Note that testing composed effects is more difficult, because it is not possible to enter the effect
/// transaction "in the middle" - only from the beginning - or to ignore some of the effects with out
/// stalling the entire downstream dependency chain.
pub struct Compose<Ev> {
    context: CapabilityContext<Never, Ev>,
}

/// A restricted context given to the closure passed to [`Compose::spawn`]. This context can only
/// update the app, not request from the shell or spawn further tasks.
pub struct ComposeContext<Ev> {
    context: CapabilityContext<Never, Ev>,
}

impl<Ev> Clone for ComposeContext<Ev> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

impl<Ev> ComposeContext<Ev> {
    /// Update the app with an event. This forwards to [`CapabilityContext::update_app`].
    pub fn update_app(&self, event: Ev)
    where
        Ev: 'static,
    {
        self.context.update_app(event);
    }
}

impl<Ev> Compose<Ev> {
    pub fn new(context: CapabilityContext<Never, Ev>) -> Self {
        Self { context }
    }

    /// Spawn a task which orchestrates across other capabilities.
    ///
    /// The argument is a closure which receives a [`ComposeContext`] which can be used to send
    /// events to the app.
    ///
    /// For example:
    /// ```
    /// # use crux_core::Command;
    /// # use crux_core::macros::Effect;
    /// # use serde::Serialize;
    /// # #[derive(Default, Clone)]
    /// # pub struct App;
    /// # #[derive(Debug, PartialEq)]
    /// # pub enum Event {
    /// #     Trigger,
    /// #     Finished(usize, usize),
    /// # }
    /// # #[derive(Default, Serialize, Debug, PartialEq)]
    /// # pub struct Model {
    /// #     pub total: usize,
    /// # }
    /// # #[derive(Effect)]
    /// # pub struct Capabilities {
    /// #     one: doctest_support::compose::capabilities::capability_one::CapabilityOne<Event>,
    /// #     two: doctest_support::compose::capabilities::capability_two::CapabilityTwo<Event>,
    /// #     compose: crux_core::compose::Compose<Event>,
    /// # }
    /// # impl crux_core::App for App {
    /// #    type Event = Event;
    /// #    type Model = Model;
    /// #    type ViewModel = Model;
    /// #    type Capabilities = Capabilities;
    /// #    type Effect = Effect;
    /// #
    ///     fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) -> Command<Effect, Event> {
    ///         match event {
    ///             Event::Trigger => caps.compose.spawn(|context| {
    ///                 let one = caps.one.clone();
    ///                 let two = caps.two.clone();
    ///
    ///                 async move {
    ///                     let (result_one, result_two) =
    ///                         futures::future::join(
    ///                             one.one_async(10),
    ///                             two.two_async(20)
    ///                         ).await;
    ///
    ///                     context.update_app(Event::Finished(result_one, result_two))
    ///                 }
    ///             }),
    ///             Event::Finished(one, two) => {
    ///                 model.total = one + two;
    ///             }
    ///         }
    ///         Command::done()
    ///     }
    /// #
    /// #    fn view(&self, _model: &Self::Model) -> Self::ViewModel {
    /// #        todo!()
    /// #    }
    /// # }
    /// ```
    pub fn spawn<F, Fut>(&self, effects_task: F)
    where
        F: FnOnce(ComposeContext<Ev>) -> Fut,
        Fut: Future<Output = ()> + 'static + Send,
        Ev: 'static,
    {
        let context = self.context.clone();
        self.context.spawn(effects_task(ComposeContext { context }));
    }
}

impl<E> Clone for Compose<E> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

impl<Ev> Capability<Ev> for Compose<Ev> {
    type Operation = Never;
    type MappedSelf<MappedEv> = Compose<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static,
    {
        Compose::new(self.context.map_event(f))
    }
}
