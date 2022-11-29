//! TODO mod docs

use crate::{Capability, Command};
use serde::{Deserialize, Serialize};

// TODO revisit this
#[derive(PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeResponse(pub String);

pub struct Time<Ef>
where
    Ef: Clone,
{
    effect: Ef,
}

impl<Ef> Time<Ef>
where
    Ef: Clone,
{
    pub fn new(effect: Ef) -> Self {
        Self { effect }
    }

    pub fn get<Ev, F>(&self, callback: F) -> Command<Ef, Ev>
    where
        Ev: 'static,
        F: Fn(TimeResponse) -> Ev + Send + Sync + 'static,
    {
        Command::new(self.effect.clone(), callback)
    }
}

impl<Ef> Capability for Time<Ef> where Ef: Clone {}
