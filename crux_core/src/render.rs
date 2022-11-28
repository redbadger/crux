use crate::Capability;

use super::Command;

pub struct Render<Ef>
where
    Ef: Clone,
{
    effect: Ef,
}

impl<Ef> Render<Ef>
where
    Ef: Clone,
{
    pub fn new(effect: Ef) -> Self {
        Self { effect }
    }

    pub fn render<Ev>(&self) -> Command<Ef, Ev>
    where
        Ev: 'static,
    {
        Command::new_without_callback(self.effect.clone())
    }
} // Public API of the capability, called by App::update.

impl<Ef> Capability for Render<Ef> where Ef: Clone {}
