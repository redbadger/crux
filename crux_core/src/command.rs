use bcs::from_bytes;
use serde::de::DeserializeOwned;

/// Command captures the intent for a side-effect. Commands are return by the [`App::update`] function.
///
/// You should never create a Command yourself, instead use one of the capabilities to create a command.
/// Command is generic over `Message` in order to carry a "callback" which will be sent to the [`App::update`]
/// function when the command has been executed, and passed the resulting data.
pub struct Command<Effect, Event> {
    pub(crate) effect: Effect, // TODO switch to `enum Effect`, so that shell knows what to do
    pub(crate) resolve: Option<Box<dyn Callback<Event> + Send + Sync>>,
}

impl<Effect, Event> Command<Effect, Event> {
    pub fn new<F, T>(effect: Effect, resolve: F) -> Self
    where
        F: Fn(T) -> Event + Send + Sync + 'static,
        Event: 'static,
        T: 'static + DeserializeOwned,
    {
        Self {
            effect,
            resolve: Some(Box::new(resolve.into_callback())),
        }
    }

    pub fn new_without_callback(effect: Effect) -> Self {
        Self {
            effect,
            resolve: None,
        }
    }

    pub fn resolve(&self, value: Vec<u8>) -> Event {
        if let Some(resolve) = &self.resolve {
            return resolve.call(value);
        }

        panic!("mismatched capability response");
    }

    /// Lift is used to convert a Command with one message type to a command with another.
    ///
    /// This is normally used when composing applications. A typical case in the top-level
    /// `update` function would look like the following:
    ///
    /// ```rust
    /// match message {
    ///     // ...
    ///     Msg::Submodule(msg) => Command::lift(
    ///             self.submodule.update(msg, &mut model.submodule),
    ///             Msg::Submodule,
    ///         ),
    ///     // ...
    /// }
    /// ```
    pub fn lift<ParentEvent, F>(
        commands: Vec<Command<Effect, Event>>,
        f: F,
    ) -> Vec<Command<Effect, ParentEvent>>
    where
        F: Fn(Event) -> ParentEvent + Sync + Send + Copy + 'static,
        Event: 'static,
        ParentEvent: 'static,
    {
        commands.into_iter().map(move |c| c.map(f)).collect()
    }

    fn map<ParentEvent, F>(self, f: F) -> Command<Effect, ParentEvent>
    where
        F: Fn(Event) -> ParentEvent + Sync + Send + Copy + 'static,
        Event: 'static,
        ParentEvent: 'static,
    {
        Command {
            effect: self.effect,
            resolve: match self.resolve {
                Some(resolve) => {
                    let callback = move |capability_response| f(resolve.call(capability_response));
                    Some(Box::new(callback.into_callback()))
                }
                None => None,
            },
        }
    }
}

pub trait Callback<Event> {
    fn call(&self, value: Vec<u8>) -> Event;
}

struct CallBackFn<T, Event> {
    function: Box<dyn Fn(T) -> Event + Send + Sync>,
}

impl<T, Event> Callback<Event> for CallBackFn<T, Event>
where
    T: DeserializeOwned,
{
    fn call(&self, value: Vec<u8>) -> Event {
        let response = from_bytes::<T>(&value).unwrap();
        (self.function)(response)
    }
}

trait IntoCallBack<T, Event> {
    fn into_callback(self) -> CallBackFn<T, Event>;
}

impl<F, T, Event> IntoCallBack<T, Event> for F
where
    F: Fn(T) -> Event + Send + Sync + 'static,
{
    fn into_callback(self) -> CallBackFn<T, Event> {
        CallBackFn {
            function: Box::new(self),
        }
    }
}
