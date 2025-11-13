pub(crate) mod channel;

mod executor;
use futures::{Stream, StreamExt as _};

pub(crate) use channel::channel;
pub(crate) use executor::{QueuingExecutor, executor_and_spawner};

#[cfg(feature = "facet_typegen")]
use crate::type_generation::facet::TypeGenError;
use crate::{Command, command::CommandOutput};
use channel::Sender;

/// Operation trait links together input and output of a side-effect.
///
/// You implement `Operation` on the payload sent by the capability to the shell using [`CapabilityContext::request_from_shell`].
///
/// For example (from `crux_http`):
///
/// ```rust,ignore
/// impl Operation for HttpRequest {
///     type Output = HttpResponse;
/// }
/// ```
pub trait Operation: Send + 'static {
    /// `Output` assigns the type this request results in.
    type Output: Send + Unpin + 'static;

    #[cfg(feature = "typegen")]
    #[allow(clippy::missing_errors_doc)]
    fn register_types(
        generator: &mut crate::type_generation::serde::TypeGen,
    ) -> crate::type_generation::serde::Result
    where
        Self: serde::Serialize + for<'de> serde::de::Deserialize<'de>,
        Self::Output: for<'de> serde::de::Deserialize<'de>,
    {
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }

    #[cfg(feature = "facet_typegen")]
    #[allow(clippy::missing_errors_doc)]
    fn register_types_facet<'a>(
        generator: &mut crate::type_generation::facet::TypeRegistry,
    ) -> Result<&mut crate::type_generation::facet::TypeRegistry, TypeGenError>
    where
        Self: facet::Facet<'a> + serde::Serialize + for<'de> serde::de::Deserialize<'de>,
        <Self as Operation>::Output: facet::Facet<'a> + for<'de> serde::de::Deserialize<'de>,
    {
        generator
            .register_type::<Self>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?
            .register_type::<Self::Output>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        Ok(generator)
    }
}

/// Initial version of capability Context which has not yet been specialized to a chosen capability
pub struct ProtoContext<Eff, Event> {
    shell_channel: Sender<Eff>,
    app_channel: Sender<Event>,
    spawner: executor::Spawner,
}

impl<Eff, Event> Clone for ProtoContext<Eff, Event> {
    fn clone(&self) -> Self {
        Self {
            shell_channel: self.shell_channel.clone(),
            app_channel: self.app_channel.clone(),
            spawner: self.spawner.clone(),
        }
    }
}

// CommandSpawner is a temporary bridge between the channel type used by the Command and the channel type
// used by the core. Once the old capability support is removed, we should be able to remove this in favour
// of the Command's ability to be hosted on a pair of channels
pub(crate) struct CommandSpawner<Effect, Event> {
    context: ProtoContext<Effect, Event>,
}

impl<Effect, Event> CommandSpawner<Effect, Event> {
    pub(crate) fn new(context: ProtoContext<Effect, Event>) -> Self {
        Self { context }
    }

    pub(crate) fn spawn(&self, mut command: Command<Effect, Event>)
    where
        Command<Effect, Event>: Stream<Item = CommandOutput<Effect, Event>>,
        Effect: Unpin + Send + 'static,
        Event: Unpin + Send + 'static,
    {
        self.context.spawner.spawn({
            let context = self.context.clone();

            async move {
                while let Some(output) = command.next().await {
                    match output {
                        CommandOutput::Effect(effect) => context.shell_channel.send(effect),
                        CommandOutput::Event(event) => context.app_channel.send(event),
                    }
                }
            }
        });
    }
}

impl<Eff, Ev> ProtoContext<Eff, Ev>
where
    Ev: 'static,
    Eff: 'static,
{
    pub(crate) fn new(
        shell_channel: Sender<Eff>,
        app_channel: Sender<Ev>,
        spawner: executor::Spawner,
    ) -> Self {
        Self {
            shell_channel,
            app_channel,
            spawner,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use static_assertions::assert_impl_all;

    use super::*;

    #[allow(dead_code)]
    enum Effect {}

    #[allow(dead_code)]
    enum Event {}

    #[derive(PartialEq, Clone, Serialize, Deserialize)]
    #[allow(dead_code)]
    struct Op {}

    impl Operation for Op {
        type Output = ();
    }

    assert_impl_all!(ProtoContext<Effect, Event>: Send, Sync);
}
