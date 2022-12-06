/// Command captures the intent for a side-effect. Commands are return by the [`App::update`] function.
///
/// You should never create a Command yourself, instead use one of the capabilities to create a command.
/// Command is generic over `Message` in order to carry a "callback" which will be sent to the [`App::update`]
/// function when the command has been executed, and passed the resulting data.
pub struct Command<Ef> {
    pub(crate) effect: Ef, // TODO switch to `enum Effect`, so that shell knows what to do
    pub(crate) resolve: Option<Resolve>,
}

pub(crate) type Resolve = Box<dyn Fn(&[u8]) + Send>;

impl<Ef> Command<Ef> {
    pub fn new<F>(effect: Ef, resolve: F) -> Self
    where
        F: Fn(&[u8]) + Send + 'static,
    {
        Self {
            effect,
            resolve: Some(Box::new(resolve)),
        }
    }

    pub fn new_without_callback(effect: Ef) -> Self {
        Self {
            effect,
            resolve: None,
        }
    }

    pub fn map_effect<NewEffect, F>(self, f: F) -> Command<NewEffect>
    where
        F: Fn(Ef) -> NewEffect + Sync + Send + Copy + 'static,
        Ef: 'static,
        NewEffect: 'static,
    {
        Command {
            effect: f(self.effect),
            resolve: self.resolve,
        }
    }
}
