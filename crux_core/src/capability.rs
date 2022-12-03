use crate::{
    channels::{channel, Receiver, Sender},
    Command,
};

// TODO docs!
pub trait Capability<Ev> {
    type MappedSelf<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static;
}

pub trait CapabilitiesFactory<App, Ef>
where
    App: crate::App,
{
    fn build(channel: Sender<Command<Ef, App::Event>>) -> App::Capabilities;
}

pub fn test_capabilities<Caps, App, Ef>() -> (App::Capabilities, Receiver<Command<Ef, App::Event>>)
where
    Caps: CapabilitiesFactory<App, Ef>,
    App: crate::App,
    App::Event: Send,
    Ef: Send + 'static,
{
    let (sender, receiver) = channel();
    (Caps::build(sender), receiver)
}
