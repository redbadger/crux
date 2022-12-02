use super::Command;
use crate::Capability;

pub struct Render<Ef>
where
    Ef: Clone,
{
    make_effect: Ef,
}

impl<Ef> Render<Ef>
where
    Ef: Clone,
{
    pub fn new(make_effect: Ef) -> Self {
        Self { make_effect }
    }

    pub fn render<Ev>(&self) -> Command<Ef, Ev>
    where
        Ev: 'static,
    {
        Command::new_without_callback(self.make_effect.clone())
    }
} // Public API of the capability, called by App::update.

impl<Ef> super::Capability for Render<Ef> where Ef: Clone {}
