//! Initializes the application to run, resulting in either onboarding or an active state.
//!
//! Triggered by: `Event::Start`
//! Outcome is one of:
//! 1. `InitializingTransition::Onboard` - the application could not fully initialize and transitions to onboarding.
//! 2. `InitializingTransition::Active(api_key)` - the application properly initialized and is active for the user.

use crux_core::{render::render, Command};
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::effects::{
    secret::{self, SecretFetchResponse},
    Effect,
};

use super::{outcome::Outcome, Event};

#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
pub enum InitializingEvent {
    #[serde(skip)]
    #[facet(skip)]
    SecretFetched(#[facet(opaque)] SecretFetchResponse),
}

/// The transition from this stage when the application is finished initializing.
#[derive(Debug)]
pub(crate) enum InitializingTransition {
    /// The application could not fully initialize and transitions to onboarding.
    Onboard,
    /// The application properly initialized and is active for the user.
    Active(String),
}

/// The model that manages initializing the application.
#[derive(Debug)]
pub struct InitializingModel;

impl InitializingModel {
    /// Starts initialization by fetching the API key secret.
    pub fn start() -> (Self, Command<Effect, Event>) {
        let cmd = secret::command::fetch(secret::API_KEY_NAME)
            .then_send(|r| Event::Initializing(InitializingEvent::SecretFetched(r)));
        (Self, cmd)
    }

    /// Processes an event during initialization from the secret, resulting in
    /// a transition to either onboarding (when it is missing), or active (when it exists).
    pub(crate) fn update(
        self,
        event: InitializingEvent,
    ) -> Outcome<Self, InitializingTransition, InitializingEvent> {
        match event {
            InitializingEvent::SecretFetched(response) => match response {
                SecretFetchResponse::Fetched(api_key) => {
                    Outcome::complete(InitializingTransition::Active(api_key), Command::done())
                }
                SecretFetchResponse::Missing(_) => {
                    Outcome::complete(InitializingTransition::Onboard, render())
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::secret;

    const API_KEY_VALUE: &str = "my_key";

    #[test]
    fn secret_missing_transitions_to_onboard() {
        let model = InitializingModel;
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Missing(secret::API_KEY_NAME.to_string()),
        ));

        let (transition, mut cmd) = outcome.expect_complete().into_parts();
        assert!(matches!(transition, InitializingTransition::Onboard));
        cmd.expect_one_effect().expect_render();
    }

    #[test]
    fn secret_fetched_transitions_to_active() {
        let model = InitializingModel;
        let outcome = model.update(InitializingEvent::SecretFetched(
            SecretFetchResponse::Fetched(API_KEY_VALUE.to_string()),
        ));

        let transition = outcome.expect_complete().into_value();
        assert_let_bind::assert_let!(InitializingTransition::Active(api_key), transition);
        assert_eq!(api_key, API_KEY_VALUE);
    }
}
