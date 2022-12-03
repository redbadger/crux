// TODO docs!
pub trait Capability<Ev> {
    type MappedSelf<MappedEv>;

    fn map_event<F, NewEvent>(&self, f: F) -> Self::MappedSelf<NewEvent>
    where
        F: Fn(NewEvent) -> Ev + Send + Sync + Copy + 'static,
        Ev: 'static,
        NewEvent: 'static;
}

pub trait Capabilities<C> {
    fn get(&self) -> &C;
}

use crate::Command;

// Don't like this factory name but the other obvious option is `Capabilities` and I'd rather let
// the individual apps use that name...
pub trait CapabilitiesFactory<App, Ef>
where
    App: crate::App,
{
    fn build(channel: crate::channels::Sender<Command<Ef, App::Event>>) -> App::Capabilities;
}
