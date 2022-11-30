//! TODO mod docs

use crux_core::{Capability, Command};
use serde::{Deserialize, Serialize};

// TODO revisit this
#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformResponse(pub String);

pub struct Platform<Ef>
where
    Ef: Clone,
{
    make_effect: Ef,
}

impl<Ef> Platform<Ef>
where
    Ef: Clone,
{
    pub fn new(make_effect: Ef) -> Self {
        Self { make_effect }
    }

    pub fn get<Ev, F>(&self, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(PlatformResponse) -> Ev + Send + Sync + 'static,
    {
        Command::new(self.make_effect.clone(), callback)
    }
}

impl<Ef> Capability for Platform<Ef> where Ef: Clone {}
