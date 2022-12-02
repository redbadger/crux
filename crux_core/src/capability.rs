// TODO docs!
// pub trait Capability {}

// pub trait Capabilities<C> {
//     fn get(&self) -> &C;
// }

use crate::Command;

// Don't like this factory name but the other obvious option is `Capabilities` and I'd rather let
// the individual apps use that name...
pub trait CapabilityFactory<App, Ef>
where
    App: crate::App,
{
    fn build(channel: std::sync::mpsc::Sender<Command<Ef, App::Event>>) -> App::Capabilities;
}
