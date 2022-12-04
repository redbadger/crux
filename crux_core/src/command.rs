use bcs::from_bytes;
use serde::de::DeserializeOwned;

/// Command captures the intent for a side-effect. Commands are return by the [`App::update`] function.
///
/// You should never create a Command yourself, instead use one of the capabilities to create a command.
/// Command is generic over `Message` in order to carry a "callback" which will be sent to the [`App::update`]
/// function when the command has been executed, and passed the resulting data.
pub struct Command<Ef, Ev> {
    pub(crate) effect: Ef, // TODO switch to `enum Effect`, so that shell knows what to do
    pub(crate) resolve: Option<Resolve<Ev>>,
}

pub(crate) enum Resolve<Ev> {
    Event(Box<dyn Callback<Ev> + Send>),
    Continue(Box<dyn Callback<()> + Send>),
}

impl<Ef, Ev> Command<Ef, Ev> {
    pub fn new<F, T>(effect: Ef, resolve: F) -> Self
    where
        F: Fn(T) -> Ev + Send + 'static,
        Ev: 'static,
        T: 'static + DeserializeOwned,
    {
        Self {
            effect,
            resolve: Some(Resolve::Event(Box::new(resolve.into_callback()))),
        }
    }

    pub fn new_without_callback(effect: Ef) -> Self {
        Self {
            effect,
            resolve: None,
        }
    }

    pub fn new_continuation<F, T>(effect: Ef, resolve: F) -> Self
    where
        F: Fn(T) + Send + 'static,
        T: 'static + DeserializeOwned,
    {
        Self {
            effect,
            resolve: Some(Resolve::Continue(Box::new(resolve.into_callback()))),
        }
    }

    pub fn map<ParentEvent, F>(self, f: F) -> Command<Ef, ParentEvent>
    where
        F: Fn(Ev) -> ParentEvent + Send + Copy + 'static,
        Ev: 'static,
        ParentEvent: 'static,
    {
        Command {
            effect: self.effect,
            resolve: match self.resolve {
                Some(Resolve::Event(resolve)) => {
                    let callback = move |capability_response: Vec<u8>| {
                        // FIXME: remove the need for this (by avoiding double deserialization)
                        let response = bcs::to_bytes(&capability_response).unwrap();

                        f(resolve.call(response))
                    };
                    Some(Resolve::Event(Box::new(callback.into_callback())))
                }
                Some(Resolve::Continue(inner)) => Some(Resolve::Continue(inner)),
                None => None,
            },
        }
    }

    pub fn map_effect<NewEffect, F>(self, f: F) -> Command<NewEffect, Ev>
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

pub trait Callback<Event> {
    fn call(&self, value: Vec<u8>) -> Event;
}

struct CallBackFn<T, Event> {
    function: Box<dyn Fn(T) -> Event + Send>,
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
    F: Fn(T) -> Event + Send + 'static,
{
    fn into_callback(self) -> CallBackFn<T, Event> {
        CallBackFn {
            function: Box::new(self),
        }
    }
}
