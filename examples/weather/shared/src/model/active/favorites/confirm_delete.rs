use crux_core::render::render;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::location::Location;
use crate::model::outcome::Outcome;

#[derive(Debug)]
pub struct ConfirmDeleteWorkflow {
    pub location: Location,
}

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum ConfirmDeleteEvent {
    Confirmed,
    Cancelled,
}

#[derive(Debug)]
pub(crate) enum ConfirmDeleteTransition {
    Confirmed(Location),
    Cancelled,
}

impl ConfirmDeleteWorkflow {
    pub fn new(location: Location) -> Self {
        Self { location }
    }

    pub(crate) fn update(
        self,
        event: ConfirmDeleteEvent,
    ) -> Outcome<Self, ConfirmDeleteTransition, ConfirmDeleteEvent> {
        match event {
            ConfirmDeleteEvent::Confirmed => {
                tracing::debug!("confirming deletion of {:?}", self.location);
                Outcome::complete(ConfirmDeleteTransition::Confirmed(self.location), render())
            }
            ConfirmDeleteEvent::Cancelled => {
                tracing::debug!("cancelling deletion of {:?}", self.location);
                Outcome::complete(ConfirmDeleteTransition::Cancelled, render())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_location() -> Location {
        Location {
            lat: 33.456_789,
            lon: -112.037_222,
        }
    }

    #[test]
    fn confirmed_completes_with_location() {
        let workflow = ConfirmDeleteWorkflow::new(test_location());
        let (transition, mut cmd) = workflow
            .update(ConfirmDeleteEvent::Confirmed)
            .expect_complete()
            .into_parts();

        assert!(matches!(
            transition,
            ConfirmDeleteTransition::Confirmed(loc) if loc == test_location()
        ));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn cancelled_completes() {
        let workflow = ConfirmDeleteWorkflow::new(test_location());
        let (transition, mut cmd) = workflow
            .update(ConfirmDeleteEvent::Cancelled)
            .expect_complete()
            .into_parts();

        assert!(matches!(transition, ConfirmDeleteTransition::Cancelled));
        cmd.expect_one_effect().expect_render();
    }
}
